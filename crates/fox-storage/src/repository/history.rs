//! Request History。

use sqlx::SqlitePool;
use uuid::Uuid;

use fox_core::model::RequestHistory;
use fox_core::Result;

use super::rows::HistoryRow;

/// 每个项目保留的历史条数上限（超出按最旧淘汰）。
///
/// 历史每条含请求/响应摘要 JSON，无限增长会持续膨胀数据库并拖慢
/// 列表查询与启动迁移，故写入时顺带按项目裁剪。
pub const HISTORY_RETENTION_PER_PROJECT: i64 = 500;

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
    // 保留策略：淘汰本项目最旧的超额历史
    sqlx::query(
        "DELETE FROM request_histories
         WHERE project_id = ? AND id NOT IN (
             SELECT id FROM request_histories WHERE project_id = ?
             ORDER BY created_at DESC LIMIT ?
         )",
    )
    .bind(&row.project_id)
    .bind(&row.project_id)
    .bind(HISTORY_RETENTION_PER_PROJECT)
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
