//! Settings（通用键值）。

use sqlx::SqlitePool;

use fox_core::Result;

pub async fn set_setting(db: &SqlitePool, key: &str, value: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO settings (key, value_json) VALUES (?, ?)
         ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json",
    )
    .bind(key)
    .bind(value)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn get_setting(db: &SqlitePool, key: &str) -> Result<Option<String>> {
    let row: Option<(String,)> = sqlx::query_as("SELECT value_json FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(db)
        .await?;
    Ok(row.map(|r| r.0))
}
