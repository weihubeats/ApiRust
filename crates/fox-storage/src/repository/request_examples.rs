//! RequestExample（请求用例）仓储：新建 / 列表 / 删除。

use sqlx::SqlitePool;
use uuid::Uuid;

use fox_core::model::RequestExample;
use fox_core::Result;

/// 新建请求用例（保存当前请求快照）。
pub async fn create_request_example(
    db: &SqlitePool,
    example: &RequestExample,
) -> Result<RequestExample> {
    sqlx::query(
        "INSERT INTO request_examples
             (id, endpoint_id, name, request_json, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(example.id.to_string())
    .bind(example.endpoint_id.to_string())
    .bind(&example.name)
    .bind(
        serde_json::to_string(&example.request)
            .map_err(|e| fox_core::AppError::Json(e))?,
    )
    .bind(example.created_at.to_rfc3339())
    .bind(example.updated_at.to_rfc3339())
    .execute(db)
    .await?;
    Ok(example.clone())
}

/// 列出接口的全部请求用例（最新在前）。
pub async fn list_request_examples(
    db: &SqlitePool,
    endpoint_id: Uuid,
) -> Result<Vec<RequestExample>> {
    let rows: Vec<RequestExampleRow> = sqlx::query_as(
        "SELECT id, endpoint_id, name, request_json, created_at, updated_at
         FROM request_examples WHERE endpoint_id = ? ORDER BY created_at DESC",
    )
    .bind(endpoint_id.to_string())
    .fetch_all(db)
    .await?;
    rows.into_iter().map(RequestExampleRow::into_model).collect()
}

/// 删除单条请求用例。
pub async fn delete_request_example(db: &SqlitePool, example_id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM request_examples WHERE id = ?")
        .bind(example_id.to_string())
        .execute(db)
        .await?;
    Ok(())
}

#[derive(sqlx::FromRow)]
struct RequestExampleRow {
    id: String,
    endpoint_id: String,
    name: String,
    request_json: String,
    created_at: String,
    updated_at: String,
}

impl RequestExampleRow {
    fn into_model(self) -> Result<RequestExample> {
        Ok(RequestExample {
            id: super::rows::parse_uuid(&self.id)?,
            endpoint_id: super::rows::parse_uuid(&self.endpoint_id)?,
            name: self.name,
            request: serde_json::from_str(&self.request_json).map_err(|e| {
                fox_core::AppError::Json(e)
            })?,
            created_at: super::rows::parse_time(&self.created_at)?,
            updated_at: super::rows::parse_time(&self.updated_at)?,
        })
    }
}