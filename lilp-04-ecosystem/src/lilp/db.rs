//! `db`模块提供了与数据库交互的函数。
//!
//! 主要包括以下函数：
//! - `get_url`: 从数据库中获取给定id的URL。
//! - `shorten`: 将给定的URL缩短，并将其存储到数据库中。
//!
//! 此模块还包含了`UrlRecord`结构体，用于表示数据库中的URL记录。

use nanoid::nanoid;
use sqlx::FromRow;

use crate::lilp::db_config::get_pgsql_pool;
use crate::lilp::error::AppError;

/// UrlRecord结构体，用于表示数据库中的URL记录
#[derive(Debug, FromRow)]
struct UrlRecord {
    #[sqlx(default)]
    id: String,
    #[sqlx(default)]
    url: String,
}

/// 从数据库中获取给定id的URL
///
/// # 参数
///
/// * `id` - 需要查询的URL的id
///
/// # 返回值
///
/// 返回一个Result，如果查询成功，返回URL的字符串，否则返回AppError
pub async fn get_url(id: &str) -> Result<String, AppError> {
    let pool = get_pgsql_pool().await;
    let ret: UrlRecord = sqlx::query_as("SELECT url FROM urls WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await?;
    Ok(ret.url)
}

/// 将给定的URL缩短，并将其存储到数据库中
///
/// # 参数
///
/// * `url` - 需要缩短的URL
///
/// # 返回值
///
/// 返回一个Result，如果操作成功，返回生成的短URL的id，否则返回AppError
pub async fn shorten(url: &str) -> Result<String, AppError> {
    let pool = get_pgsql_pool().await;
    #[cfg(test)]
    let mut test_num = 0;
    loop {
        #[cfg(test)]
        let id = {
            test_num += 1;
            if test_num > 3 {
                format!("test{}", test_num)
            } else {
                "test0".to_string()
            }
        };

        #[cfg(not(test))]
        let id = nanoid!(6);

        let result = sqlx::query_as::<_, UrlRecord>(
            "INSERT INTO urls (id, url) VALUES ($1, $2) ON CONFLICT(url) DO UPDATE SET url=EXCLUDED.url RETURNING id",
        )
            .bind(&id)
            .bind(url)
            .fetch_one(pool)
            .await;

        match result {
            Ok(ret) => return Ok(ret.id),
            Err(sqlx::Error::Database(db_err)) if db_err.constraint() == Some("urls_pkey") => {
                continue;
            }
            Err(e) => return Err(AppError::DatabaseError(e)),
        }
    }
}

#[cfg(test)]
mod pgsql_tests {
    use super::*;

    /// 测试shorten函数
    #[ignore]
    #[tokio::test]
    async fn test_shorten() -> anyhow::Result<()> {
        let url = "https://www.rust-lang.org/3";
        let id = shorten(url).await?;
        Ok(())
    }
}
