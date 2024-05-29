## 重写聊天服务器
使用broadcast::channel替换mpsc::channel
```shell
cd lilp-04-ecosystem
RUSTFLAGS="--cfg tokio_unstable" cargo run --example lilp_chat
tokio-console
telnet 127.0.0.1 8080
```

## 重写url shortener


1. 如果生成的 id 重复，而产生数据库错误，则重新生成一个 id。（lilp-04-ecosystem/src/lilp/db.rs:93）
2. 使用 thiserror 进行错误处理（为你定义的 error 实现 IntoResponse）。（lilp-04-ecosystem/src/lilp/handler.rs:35）

### 运行
需要先设置环境变量 export DATABASE_RUST_BOOTCAMP="postgres://postgres:password@ip:port/rust_bootcamp"
```shell
cd lilp-04-ecosystem
cargo run --example lilp_shortener
```
```shell
curl -X POST http://localhost:9876/ \
-H "Content-Type: application/json" \
-d '{"url": "https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/422"}'
```
```shell
curl -v http://localhost:9876/m2Gaxi
```
