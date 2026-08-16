//! Request History。

use sqlx::SqlitePool;
use uuid::Uuid;

use fox_core::model::RequestHistory;
use fox_core::Result;

use super::rows::HistoryRow;

pub async fn save_request_history(db: &SqlitePool, model: &RequestHistory) -> Result<()> {
    let row = HistoryRow::from_model(model);
    sqlx::query(
        "INSERT INTO request_histories
         (id, project_id, endpoint_id, method, url, status, duration_ms, request_summary_json, response_summary_json, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&row.id)
    .bind(&row.project_id)
    .bind(&row.endpoint_id)
    .bind(&row.method)
    .bind(&row.url)
    .bind(row.status)
    .bind(row.duration_ms)
    .bind(&row.request_summary_json)
    .bind(&row.response_summary_json)
    .bind(row.created_at.clone())
    .execute(db)
    .await?;
    Ok(())
}

pub async fn list_request_histories(
    db: &SqlitePool,
    project_id: Uuid,
    limit: i64,
) -> Result<Vec<RequestHistory>> {
    let rows: Vec<HistoryRow> = sqlx::query_as(
        "SELECT id, project_id, endpoint_id, method, url, status, duration_ms,
                request_summary_json, response_summary_json, created_at
         FROM request_histories
         WHERE project_id = ?
         ORDER BY created_at DESC
         LIMIT ?",
    )
    .bind(project_id.to_string())
    .bind(limit)
    .fetch_all(db)
    .await?;
    rows.into_iter().map(HistoryRow::into_model).collect()
}
