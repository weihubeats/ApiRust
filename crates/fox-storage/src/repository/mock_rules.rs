//! Mock Rules（自定义 Mock 规则）。

use sqlx::SqlitePool;
use uuid::Uuid;

use fox_core::model::MockRule;
use fox_core::{AppError, Result};

use super::rows::MockRuleRow;

pub async fn create_mock_rule(
    db: &SqlitePool,
    project_id: Uuid,
    rule: &MockRule,
) -> Result<MockRule> {
    let row = MockRuleRow::from_model(rule);
    sqlx::query(
        "INSERT INTO mock_rules
         (id, project_id, endpoint_id, name, method, path, match_query_json, match_headers_json,
          response_status, response_headers_json, response_body_template, delay_ms, enabled, priority,
          created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
            project_id = excluded.project_id,
            endpoint_id = excluded.endpoint_id,
            name = excluded.name,
            method = excluded.method,
            path = excluded.path,
            match_query_json = excluded.match_query_json,
            match_headers_json = excluded.match_headers_json,
            response_status = excluded.response_status,
            response_headers_json = excluded.response_headers_json,
            response_body_template = excluded.response_body_template,
            delay_ms = excluded.delay_ms,
            enabled = excluded.enabled,
            priority = excluded.priority,
            updated_at = excluded.updated_at",
    )
    .bind(&row.id)
    .bind(&row.project_id)
    .bind(&row.endpoint_id)
    .bind(&row.name)
    .bind(&row.method)
    .bind(&row.path)
    .bind(&row.match_query_json)
    .bind(&row.match_headers_json)
    .bind(row.response_status)
    .bind(&row.response_headers_json)
    .bind(&row.response_body_template)
    .bind(row.delay_ms)
    .bind(row.enabled)
    .bind(row.priority)
    .bind(row.created_at.clone())
    .bind(row.updated_at.clone())
    .execute(db)
    .await?;
    let _ = project_id;
    Ok(rule.clone())
}

pub async fn list_mock_rules(db: &SqlitePool, project_id: Uuid) -> Result<Vec<MockRule>> {
    let rows: Vec<MockRuleRow> = sqlx::query_as(
        "SELECT id, project_id, endpoint_id, name, method, path, match_query_json, match_headers_json,
                response_status, response_headers_json, response_body_template, delay_ms, enabled, priority,
                created_at, updated_at
         FROM mock_rules WHERE project_id = ? ORDER BY priority DESC, created_at",
    )
    .bind(project_id.to_string())
    .fetch_all(db)
    .await?;
    rows.into_iter().map(MockRuleRow::into_model).collect()
}

pub async fn update_mock_rule(db: &SqlitePool, rule: &MockRule) -> Result<MockRule> {
    let row = MockRuleRow::from_model(rule);
    let result = sqlx::query(
        "UPDATE mock_rules SET name = ?, method = ?, path = ?, match_query_json = ?, match_headers_json = ?,
                response_status = ?, response_headers_json = ?, response_body_template = ?, delay_ms = ?,
                enabled = ?, priority = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(&row.name)
    .bind(&row.method)
    .bind(&row.path)
    .bind(&row.match_query_json)
    .bind(&row.match_headers_json)
    .bind(row.response_status)
    .bind(&row.response_headers_json)
    .bind(&row.response_body_template)
    .bind(row.delay_ms)
    .bind(row.enabled)
    .bind(row.priority)
    .bind(row.updated_at.clone())
    .bind(&row.id)
    .execute(db)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("Mock 规则（{}）", rule.id)));
    }
    Ok(rule.clone())
}

pub async fn delete_mock_rule(db: &SqlitePool, rule_id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM mock_rules WHERE id = ?")
        .bind(rule_id.to_string())
        .execute(db)
        .await?;
    Ok(())
}

/// 带 id：原样写入 Mock 规则。
pub async fn save_mock_rule(db: &SqlitePool, rule: &MockRule) -> Result<()> {
    create_mock_rule(db, rule.project_id, rule)
        .await
        .map(|_| ())
}
