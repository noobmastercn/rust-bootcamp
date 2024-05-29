//! 该案例实现了一个简单的基于 TCP 的聊天服务器。
//! 主要功能包括：用户连接、断开连接、发送和接收消息的处理。
use anyhow::Result;
use console_subscriber::ConsoleLayer;
use dashmap::DashMap;
use futures::{stream::SplitStream, SinkExt, StreamExt};
use std::{fmt, net::SocketAddr, sync::Arc};
use tokio::sync::watch;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::broadcast,
};
use tokio_util::codec::{Framed, LinesCodec};
use tracing::{info, level_filters::LevelFilter, warn};
use tracing_subscriber::{
    fmt::Layer as FmtLayer, layer::SubscriberExt, util::SubscriberInitExt, Layer as _,
};

const MAX_MESSAGES: usize = 128;

/// 保存服务器状态，包括在线的peer和消息发送者
#[derive(Debug)]
struct State {
    peers: DashMap<SocketAddr, String>,
    sender: broadcast::Sender<Arc<Message>>,
}

impl State {
    /// 创建一个新的State实例
    fn new() -> Self {
        let (sender, _) = broadcast::channel(MAX_MESSAGES);
        State {
            peers: DashMap::new(),
            sender,
        }
    }
}

/// 表示一个连接的peer
#[derive(Debug)]
struct Peer {
    username: String,
    stream: SplitStream<Framed<TcpStream, LinesCodec>>,
}

/// 表示聊天消息的枚举类型
#[derive(Debug, Clone)]
enum Message {
    /// 用户加入聊天室
    UserJoined(String),
    /// 用户离开聊天室
    UserLeft(String),
    /// 用户发送的聊天消息
    Chat { sender: String, content: String },
}

/// 主函数，启动聊天服务器
///
/// # 返回
/// 如果成功则返回 `Ok(())`，否则返回错误。
#[tokio::main]
async fn main() -> Result<()> {
    let (console_layer, server) = ConsoleLayer::builder().build();
    let fmt_layer = FmtLayer::new().with_filter(LevelFilter::INFO);

    // 初始化日志记录
    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(console_layer)
        .init();

    // 启动控制台服务器
    tokio::spawn(async move {
        server.serve().await.unwrap();
    });

    let addr = "0.0.0.0:8080";
    let listener = TcpListener::bind(addr).await?;
    info!("Starting chat server on {}", addr);
    let state = Arc::new(State::new());

    loop {
        let (stream, addr) = listener.accept().await?;
        info!("Accepted connection from: {}", addr);
        let state_cloned = state.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_client(state_cloned, addr, stream).await {
                warn!("Failed to handle client {}: {}", addr, e);
            }
        });
    }
}

/// 处理客户端连接的函数
///
/// # 参数
/// - `state` - 包含当前服务器状态的共享指针
/// - `addr` - 客户端的套接字地址
/// - `stream` - 客户端的 TCP 流
///
/// # 返回
/// 如果成功则返回 `Ok(())`，否则返回错误。
async fn handle_client(state: Arc<State>, addr: SocketAddr, stream: TcpStream) -> Result<()> {
    let mut stream = Framed::new(stream, LinesCodec::new());
    stream.send("Enter your username:").await?;

    let username = match stream.next().await {
        Some(Ok(username)) => username,
        Some(Err(e)) => return Err(e.into()),
        None => return Ok(()),
    };

    // 用于关闭客户端peer发送流
    let (_shutdown_tx, shutdown_rx) = watch::channel(());
    let mut peer = state.add(addr, username, stream, shutdown_rx).await;

    let message = Arc::new(Message::user_joined(&peer.username));
    info!("{}", message);
    state.broadcast(message.clone()).await;

    while let Some(line) = peer.stream.next().await {
        let line = match line {
            Ok(line) => line,
            Err(e) => {
                warn!("Failed to read line from {}: {}", addr, e);
                break;
            }
        };
        let message = Arc::new(Message::chat(&peer.username, line));
        state.broadcast(message.clone()).await;
    }

    state.peers.remove(&addr);

    let message = Arc::new(Message::user_left(&peer.username));
    info!("{}", message);

    state.broadcast(message).await;

    // 发送消息关闭客户端peer发送流 不发送也可以 shutdown_tx出了作用域会自动关闭select! 中的 shutdown_rx.changed()就结束了 直接break
    // let _ = _shutdown_tx.send(());

    Ok(())
}

impl State {
    /// 广播消息给所有的peer
    ///
    /// # 参数
    /// - `message` - 要广播的消息
    async fn broadcast(&self, message: Arc<Message>) {
        let _ = self.sender.send(message);
    }

    /// 添加新的peer到状态中
    ///
    /// # 参数
    /// - `addr` - 客户端的套接字地址
    /// - `username` - 客户端的用户名
    /// - `stream` - 客户端的 TCP 流
    /// - `shutdown_rx` - 用于接收关闭信号的接收器
    ///
    /// # 返回
    /// 返回一个新的 `Peer` 实例
    async fn add(
        &self,
        addr: SocketAddr,
        username: String,
        stream: Framed<TcpStream, LinesCodec>,
        mut shutdown_rx: watch::Receiver<()>,
    ) -> Peer {
        self.peers.insert(addr, username.clone());

        let mut receiver = self.sender.subscribe();
        let (mut stream_sender, stream_receiver) = stream.split();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_rx.changed() => {
                        break;
                    }
                    result = receiver.recv() => {
                        match result {
                            Ok(message) => {
                                if let Err(e) = stream_sender.send(message.to_string()).await {
                                    warn!("Failed to send message to {}: {}", addr, e);
                                    break;
                                }
                            }
                            Err(e) => {
                                warn!("Failed to receive message for {}: {}", addr, e);
                                break;
                            }
                        }
                    }
                }
            }
        });

        Peer {
            username,
            stream: stream_receiver,
        }
    }
}

impl Message {
    /// 创建用户加入的消息
    ///
    /// # 参数
    /// - `username` - 加入用户的用户名
    ///
    /// # 返回
    /// 返回一个新的 `Message::UserJoined` 实例
    fn user_joined(username: &str) -> Self {
        let content = format!("{} has joined the chat", username);
        Self::UserJoined(content)
    }

    /// 创建用户离开的消息
    ///
    /// # 参数
    /// - `username` - 离开用户的用户名
    ///
    /// # 返回
    /// 返回一个新的 `Message::UserLeft` 实例
    fn user_left(username: &str) -> Self {
        let content = format!("{} has left the chat", username);
        Self::UserLeft(content)
    }

    /// 创建聊天消息
    ///
    /// # 参数
    /// - `sender` - 发送消息的用户名
    /// - `content` - 消息内容
    ///
    /// # 返回
    /// 返回一个新的 `Message::Chat` 实例
    fn chat(sender: impl Into<String>, content: impl Into<String>) -> Self {
        Self::Chat {
            sender: sender.into(),
            content: content.into(),
        }
    }
}

impl fmt::Display for Message {
    /// 实现消息的格式化输出
    ///
    /// # 参数
    /// - `f` - 格式化器
    ///
    /// # 返回
    /// 返回格式化结果
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UserJoined(content) => write!(f, "[{}]", content),
            Self::UserLeft(content) => write!(f, "[{} :(]", content),
            Self::Chat { sender, content } => write!(f, "{}: {}", sender, content),
        }
    }
}
