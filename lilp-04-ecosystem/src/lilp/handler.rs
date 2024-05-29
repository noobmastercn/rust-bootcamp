use crate::lilp::db;
use crate::lilp::error::AppError;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::Json;
use http::header::LOCATION;
use http::{HeaderMap, StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// AppState结构体，包含了应用的状态信息
#[derive(Debug, Clone)]
pub struct AppState {
    pub listen_addr: Arc<String>,
}

/// ShortenReq结构体，用于接收缩短URL请求的数据
#[derive(Debug, Deserialize)]
pub struct ShortenReq {
    url: String,
}

/// ShortenRes结构体，用于返回缩短URL的结果
#[derive(Debug, Serialize)]
struct ShortenRes {
    url: String,
}

/// shorten函数，用于处理缩短URL的请求
/// 接收一个AppState的状态和一个ShortenReq的请求数据
/// 返回一个Result，包含了一个可以转换为响应的类型，或者一个AppError
pub async fn shorten(
    State(state): State<AppState>,
    Json(data): Json<ShortenReq>,
) -> Result<impl IntoResponse, AppError> {
    let short_url_id = db::shorten(&data.url).await?;
    let body = Json(ShortenRes {
        url: format!("http://{}/{}", state.listen_addr, short_url_id),
    });
    Ok((StatusCode::CREATED, body))
}

/// redirect函数，用于处理重定向的请求
/// 接收一个id作为路径参数
/// 返回一个Result，包含了一个可以转换为响应的类型，或者一个AppError
pub async fn redirect(Path(id): Path<String>) -> Result<impl IntoResponse, AppError> {
    let full_url = db::get_url(&id).await?;
    let mut headers = HeaderMap::new();
    headers.insert(LOCATION, full_url.parse()?);
    Ok((StatusCode::PERMANENT_REDIRECT, headers))
}
