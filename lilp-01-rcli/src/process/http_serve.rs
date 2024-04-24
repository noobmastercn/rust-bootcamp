use anyhow::Result;
use askama_axum::Template;
use axum::response::{Html, IntoResponse};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Router,
};
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::fs;
use tower_http::services::ServeDir;
use tracing::{info, warn};

#[derive(Debug)]
struct HttpServeState {
    path: PathBuf,
}

pub async fn process_http_serve(path: PathBuf, port: u16) -> Result<()> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Serving {:?} on {}", path, addr);

    let state = HttpServeState { path: path.clone() };
    // axum router
    let router = Router::new()
        .nest_service("/tower", ServeDir::new(path))
        .route("/", get(file_index_handler))
        .route("/*path", get(file_handler))
        .with_state(Arc::new(state));

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;
    Ok(())
}

#[derive(Template)]
#[template(path = "directory.html")]
struct DirectoryTemplate {
    path: String,
    files: Vec<FileInfo>,
}

#[derive(Debug)]
struct FileInfo {
    name: String,
    path: String,
}

async fn file_index_handler(state: State<Arc<HttpServeState>>) -> impl IntoResponse {
    file_handler(state, Path(".".to_string())).await
}

async fn file_handler(
    State(state): State<Arc<HttpServeState>>,
    Path(req_path): Path<String>,
) -> impl IntoResponse {
    let full_path = state.path.join(&req_path);
    info!(
        "state.path: {:?}, req_path: {:?}, full_path: {:?}",
        state.path, req_path, full_path
    );
    if full_path.is_dir() {
        let mut files = Vec::new();
        // 添加返回上一级目录的链接
        if req_path != "." {
            let parent_path = std::path::Path::new(&req_path)
                .parent()
                .map_or(".".to_string(), |p| p.to_str().unwrap_or(".").to_string());
            files.push(FileInfo {
                name: "../".to_string(),
                path: "/".to_owned() + &parent_path,
            });
        }
        if let Ok(mut entries) = fs::read_dir(&full_path).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if let Ok(file_name) = entry.file_name().into_string() {
                    // 构建正确的文件路径，确保包括所有上级目录
                    let file_path = if req_path.ends_with('/') {
                        format!("{}{}", req_path, file_name)
                    } else {
                        format!("{}/{}", req_path, file_name)
                    };

                    let display_path = if entry.path().is_dir() {
                        // 如果是目录，则在显示名称末尾添加'/'
                        FileInfo {
                            name: file_name + "/",
                            path: file_path,
                        }
                    } else {
                        println!("文件 file_path: {:?}", "/".to_owned() + &file_path);
                        FileInfo {
                            name: file_name,
                            path: "/".to_owned() + &file_path,
                        }
                    };
                    files.push(display_path);
                }
            }
        }
        Html(
            DirectoryTemplate {
                path: req_path,
                files,
            }
            .render()
            .unwrap_or_else(|_| "Template rendering error".to_string()),
        )
        .into_response()
    } else if full_path.exists() {
        match fs::read_to_string(full_path).await {
            Ok(content) => {
                info!("Read {} bytes", content.len());
                Html(content).into_response()
            }
            Err(e) => {
                warn!("Error reading file: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
            }
        }
    } else {
        (
            StatusCode::NOT_FOUND,
            format!("File {} not found", req_path),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_file_handler() {
        let state = Arc::new(HttpServeState {
            path: PathBuf::from("."),
        });
        let (status, content) = file_handler(State(state), Path("Cargo.toml".to_string())).await;
        assert_eq!(status, StatusCode::OK);
        assert!(content.trim().starts_with("[package]"));
    }
}
