//! 数据库连接、路径与迁移。

use std::path::{Path, PathBuf};

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;

use fox_core::{AppError, Result};

/// {SystemDataDir}/RustFox（开发构建用 RustFox-dev，与正式版数据隔离：
/// 避免 tauri dev 跑过更新的迁移后，旧正式版打开同一数据库因迁移版本
/// 校验失败而启动即退出）
pub fn data_dir() -> PathBuf {
    let sub = if cfg!(debug_assertions) {
        "RustFox-dev"
    } else {
        "RustFox"
    };
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(sub)
}

/// 日志目录 {SystemDataDir}/RustFox/logs
pub fn log_dir() -> PathBuf {
    data_dir().join("logs")
}

/// 数据库文件路径。
pub fn database_path() -> PathBuf {
    data_dir().join("rustfox.db")
}

/// 建立连接并执行迁移。
pub async fn init_db(path: &Path) -> Result<SqlitePool> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let pool = connect(path).await?;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| AppError::Database(sqlx::Error::Protocol(e.to_string())))?;
    Ok(pool)
}

/// 只建连接，不跑迁移（测试用）。
pub async fn connect(path: &Path) -> Result<SqlitePool> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;
    Ok(pool)
}

/// 内存数据库（测试用）。
pub async fn memory_pool() -> Result<SqlitePool> {
    let options = SqliteConnectOptions::new()
        .filename(":memory:")
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| AppError::Database(sqlx::Error::Protocol(e.to_string())))?;
    Ok(pool)
}
