//! ResponseExample（响应示例）。

use sqlx::SqlitePool;
use uuid::Uuid;

use fox_core::model::ResponseExample;
use fox_core::Result;

use super::rows::ResponseExampleRow;

pub async fn create_response_example(
    db: &SqlitePool,
    endpoint_id: Uuid,
    example: &ResponseExample,
) -> Result<ResponseExample> {
    let row = ResponseExampleRow::from_model(example);
    sqlx::query(
        "INSERT INTO response_examples
             (id, endpoint_id, name, status, headers_json, body, content_type, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&row.id)
    .bind(&row.endpoint_id)
    .bind(&row.name)
    .bind(row.status)
    .bind(row.headers_json.clone())
    .bind(&row.body)
    .bind(&row.content_type)
    .bind(row.created_at.clone())
    .bind(row.updated_at.clone())
    .execute(db)
    .await?;
    let _ = endpoint_id;
    Ok(example.clone())
}

pub async fn list_response_examples(
    db: &SqlitePool,
    endpoint_id: Uuid,
) -> Result<Vec<ResponseExample>> {
    let rows: Vec<ResponseExampleRow> = sqlx::query_as(
        "SELECT id, endpoint_id, name, status, headers_json, body, content_type, created_at, updated_at
         FROM response_examples WHERE endpoint_id = ? ORDER BY created_at",
    )
    .bind(endpoint_id.to_string())
    .fetch_all(db)
    .await?;
    rows.into_iter()
        .map(ResponseExampleRow::into_model)
        .collect()
}

/// 删除单条响应示例（M10 示例管理）。
pub async fn delete_response_example(db: &SqlitePool, example_id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM response_examples WHERE id = ?")
        .bind(example_id.to_string())
        .execute(db)
        .await?;
    Ok(())
}

/// 删除某接口的全部响应示例（导入覆盖时使用）。
pub async fn delete_response_examples(db: &SqlitePool, endpoint_id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM response_examples WHERE endpoint_id = ?")
        .bind(endpoint_id.to_string())
        .execute(db)
        .await?;
    Ok(())
}

/// 带 id：原样写入响应示例。
pub async fn save_response_example(db: &SqlitePool, example: &ResponseExample) -> Result<()> {
    create_response_example(db, example.endpoint_id, example)
        .await
        .map(|_| ())
}
