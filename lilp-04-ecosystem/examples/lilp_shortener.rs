use axum::routing::{get, post};
use axum::Router;
use ecosystem::handler::{redirect, shorten, AppState};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::fmt::Layer as FmtLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

const LISTEN_ADDR: &str = "127.0.0.1:9876";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let state = AppState {
        listen_addr: Arc::new(LISTEN_ADDR.to_string()),
    };

    let layer = FmtLayer::new().with_filter(LevelFilter::INFO);
    tracing_subscriber::registry().with(layer).init();

    let listener = TcpListener::bind(LISTEN_ADDR).await?;
    info!("Listening on: {}", LISTEN_ADDR);

    let app = Router::new()
        .route("/", post(shorten))
        .route("/:id", get(redirect))
        .with_state(state);

    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}
