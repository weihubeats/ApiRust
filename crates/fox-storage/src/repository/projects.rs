//! Project CRUD。

use std::collections::HashMap;

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use fox_core::model::Project;
use fox_core::{AppError, Result};

use super::rows::ProjectRow;

pub async fn create_project(db: &SqlitePool, name: &str, description: &str) -> Result<Project> {
    let now = Utc::now();
    let model = Project {
        id: Uuid::new_v4(),
        name: name.to_string(),
        description: description.to_string(),
        variables: HashMap::new(),
        created_at: now,
        updated_at: now,
    };
    let row = ProjectRow::from_model(&model);
    sqlx::query(
        "INSERT INTO projects (id, name, description, variables_json, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&row.id)
    .bind(&row.name)
    .bind(&row.description)
    .bind(row.variables_json.clone())
    .bind(row.created_at.clone())
    .bind(row.updated_at.clone())
    .execute(db)
    .await?;
    Ok(model)
}

pub async fn list_projects(db: &SqlitePool) -> Result<Vec<Project>> {
    let rows: Vec<ProjectRow> = sqlx::query_as(
        "SELECT id, name, description, variables_json, created_at, updated_at
         FROM projects ORDER BY created_at",
    )
    .fetch_all(db)
    .await?;
    rows.into_iter().map(ProjectRow::into_model).collect()
}

pub async fn get_project(db: &SqlitePool, project_id: Uuid) -> Result<Project> {
    let row: Option<ProjectRow> = sqlx::query_as(
        "SELECT id, name, description, variables_json, created_at, updated_at
         FROM projects WHERE id = ?",
    )
    .bind(project_id.to_string())
    .fetch_optional(db)
    .await?;
    row.map(ProjectRow::into_model)
        .transpose()?
        .ok_or_else(|| AppError::NotFound(format!("项目（{project_id}）")))
}

pub async fn update_project(db: &SqlitePool, project: &Project) -> Result<Project> {
    let mut updated = project.clone();
    updated.updated_at = Utc::now();
    let row = ProjectRow::from_model(&updated);
    let result = sqlx::query(
        "UPDATE projects SET name = ?, description = ?, variables_json = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(&row.name)
    .bind(&row.description)
    .bind(row.variables_json.clone())
    .bind(row.updated_at.clone())
    .bind(&row.id)
    .execute(db)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("项目（{}）", project.id)));
    }
    Ok(updated)
}

pub async fn delete_project(db: &SqlitePool, project_id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM projects WHERE id = ?")
        .bind(project_id.to_string())
        .execute(db)
        .await?;
    Ok(())
}

/// 备份恢复：按给定 id 原样写入项目。
/// 带 id：原样写入项目（upsert，同一 id 重复保存时更新而非报主键冲突）。
pub async fn save_project(db: &SqlitePool, project: &Project) -> Result<()> {
    let row = ProjectRow::from_model(project);
    sqlx::query(
        "INSERT INTO projects (id, name, description, variables_json, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            description = excluded.description,
            variables_json = excluded.variables_json,
            updated_at = excluded.updated_at",
    )
    .bind(&row.id)
    .bind(&row.name)
    .bind(&row.description)
    .bind(&row.variables_json)
    .bind(row.created_at.clone())
    .bind(row.updated_at.clone())
    .execute(db)
    .await?;
    Ok(())
}
