//! 这个模块负责初始化和管理 PostgreSQL 数据库连接池。
//! 主要功能包括：
//! - 通过环境变量 `DATABASE_RUST_BOOTCAMP` 获取数据库连接 URL。
//! - 检查指定的数据库是否存在，如果不存在则创建它。
//! - 在数据库中创建必要的表（例如：`urls` 表）。
//! - 提供异步函数 `get_pgsql_pool` 来获取数据库连接池。

use sqlx::{postgres, PgPool, Pool, Postgres};
use tokio::sync::OnceCell;
use tokio_postgres::{Client, NoTls};
use tracing::info;

/// 全局的 PostgreSQL 连接池。
pub static PGSQL_POOL: OnceCell<PgPool> = OnceCell::const_new();

/// 获取 PostgreSQL 连接池的异步函数。
/// 如果连接池尚未初始化，则进行初始化。
/// 初始化过程中会检查数据库是否存在，如果不存在则创建它。
/// 需要先设置环境变量 export DATABASE_RUST_BOOTCAMP="postgres://postgres:password@ip:port/rust_bootcamp"
///
/// # 返回
///
/// 返回一个指向 PostgreSQL 连接池的静态引用。
///
/// # 示例
///
/// ```rust,ignore
/// let pool = get_pgsql_pool().await;
/// ```
pub async fn get_pgsql_pool() -> &'static Pool<Postgres> {
    PGSQL_POOL
        .get_or_init(|| async {
            let database_url = std::env::var("DATABASE_RUST_BOOTCAMP").expect(
                "Please set the database URL in the environment variable DATABASE_RUST_BOOTCAMP.",
            );
            // 检查数据库是否存在
            if let Err(_) = check_database_exists(&database_url).await {
                // 创建数据库
                if let Err(e) = create_database(&database_url).await {
                    panic!("Failed to create database: {}", e);
                }
            }

            let pgsql_pool = postgres::PgPoolOptions::new()
                .max_connections(1)
                .connect(&database_url)
                .await
                .expect("Failed to create pool.");
            info!("Database connection pool created.");
            pgsql_pool
        })
        .await
}

/// 检查数据库是否存在的异步函数。
/// 如果数据库存在，则创建必要的表。
///
/// # 参数
///
/// - `url`: 数据库连接 URL。
///
/// # 返回
///
/// 返回一个 `anyhow::Result` 类型，表示检查结果。
async fn check_database_exists(url: &str) -> anyhow::Result<()> {
    let (client, connection) = tokio_postgres::connect(url, NoTls).await?;
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Connection error: {}", e);
        }
    });
    client.simple_query("SELECT 1").await?;
    // 到这里说明数据库存在，执行创建表的操作
    create_table(client).await?;
    Ok(())
}

/// 创建数据库的异步函数。
///
/// # 参数
///
/// - `url`: 数据库连接 URL。
///
/// # 返回
///
/// 返回一个 `anyhow::Result` 类型，表示创建结果。
async fn create_database(url: &str) -> anyhow::Result<()> {
    // 将数据库名从 URL 中提取出来
    let db_url = url
        .split('/')
        .take(url.split('/').count() - 1)
        .collect::<Vec<&str>>()
        .join("/");
    println!("db_url: {}", db_url);
    let db_name = url.split('/').last().unwrap();
    println!("db_name: {}", db_name);

    let (client, connection) = tokio_postgres::connect(&db_url, NoTls).await?;

    // 启动一个异步任务来处理连接的后台任务
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Connection error: {}", e);
        }
    });

    // 创建数据库
    client
        .simple_query(&format!("CREATE DATABASE {}", db_name))
        .await?;
    // 到这里说明数据库存在，执行创建表的操作
    create_table(client).await?;
    Ok(())
}

/// 创建表的异步函数。
///
/// # 参数
///
/// - `client`: PostgreSQL 客户端。
///
/// # 返回
///
/// 返回一个 `anyhow::Result` 类型，表示创建结果。
async fn create_table(client: Client) -> anyhow::Result<()> {
    client
        .simple_query(
            r#"
        CREATE TABLE IF NOT EXISTS urls (
            id CHAR(6) PRIMARY KEY,
            url TEXT NOT NULL UNIQUE
        )
        "#,
        )
        .await?;
    Ok(())
}

#[cfg(test)]
mod pgsql_tests {
    use super::*;

    /// 测试获取 PostgreSQL 连接池的异步函数。
    ///
    /// # 返回
    ///
    /// 返回一个 `anyhow::Result` 类型，表示测试结果。
    #[ignore]
    #[tokio::test]
    async fn test_pgsql_pool() -> anyhow::Result<()> {
        let _conn = get_pgsql_pool().await;
        Ok(())
    }
}
