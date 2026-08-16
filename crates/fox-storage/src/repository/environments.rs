//! Environment CRUD。

use std::collections::HashMap;

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use fox_core::model::Environment;
use fox_core::{AppError, Result};

use super::rows::EnvironmentRow;

pub async fn create_environment(
    db: &SqlitePool,
    project_id: Uuid,
    name: &str,
    variables: &HashMap<String, String>,
) -> Result<Environment> {
    let now = Utc::now();
    let model = Environment {
        id: Uuid::new_v4(),
        project_id,
        name: name.to_string(),
        variables: variables.clone(),
        created_at: now,
        updated_at: now,
    };
    let row = EnvironmentRow::from_model(&model);
    sqlx::query(
        "INSERT INTO environments (id, project_id, name, variables_json, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&row.id)
    .bind(&row.project_id)
    .bind(&row.name)
    .bind(row.variables_json.clone())
    .bind(row.created_at.clone())
    .bind(row.updated_at.clone())
    .execute(db)
    .await?;
    Ok(model)
}

pub async fn get_environment(db: &SqlitePool, environment_id: Uuid) -> Result<Environment> {
    let row: Option<EnvironmentRow> = sqlx::query_as(
        "SELECT id, project_id, name, variables_json, created_at, updated_at
         FROM environments WHERE id = ?",
    )
    .bind(environment_id.to_string())
    .fetch_optional(db)
    .await?;
    row.map(EnvironmentRow::into_model)
        .transpose()?
        .ok_or_else(|| AppError::NotFound(format!("环境（{environment_id}）")))
}

pub async fn update_environment(db: &SqlitePool, environment: &Environment) -> Result<Environment> {
    let mut updated = environment.clone();
    updated.updated_at = Utc::now();
    let row = EnvironmentRow::from_model(&updated);
    let result = sqlx::query(
        "UPDATE environments SET name = ?, variables_json = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&row.name)
    .bind(row.variables_json.clone())
    .bind(row.updated_at.clone())
    .bind(&row.id)
    .execute(db)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("环境（{}）", environment.id)));
    }
    Ok(updated)
}

pub async fn delete_environment(db: &SqlitePool, environment_id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM environments WHERE id = ?")
        .bind(environment_id.to_string())
        .execute(db)
        .await?;
    Ok(())
}

pub async fn list_environments(db: &SqlitePool, project_id: Uuid) -> Result<Vec<Environment>> {
    let rows: Vec<EnvironmentRow> = sqlx::query_as(
        "SELECT id, project_id, name, variables_json, created_at, updated_at
         FROM environments WHERE project_id = ? ORDER BY created_at",
    )
    .bind(project_id.to_string())
    .fetch_all(db)
    .await?;
    rows.into_iter().map(EnvironmentRow::into_model).collect()
}

/// 带 id：原样写入环境（upsert，同一 id 重复保存时更新而非报主键冲突）。
pub async fn save_environment(db: &SqlitePool, row: &Environment) -> Result<()> {
    let row = EnvironmentRow::from_model(row);
    sqlx::query(
        "INSERT INTO environments (id, project_id, name, variables_json, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
            project_id = excluded.project_id,
            name = excluded.name,
            variables_json = excluded.variables_json,
            updated_at = excluded.updated_at",
    )
    .bind(&row.id)
    .bind(&row.project_id)
    .bind(&row.name)
    .bind(&row.variables_json)
    .bind(row.created_at.clone())
    .bind(row.updated_at.clone())
    .execute(db)
    .await?;
    Ok(())
}
