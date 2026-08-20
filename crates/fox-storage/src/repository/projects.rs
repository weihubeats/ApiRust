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
    // 新项目排在末尾（sort_order 取当前最大值 + 1）。
    let sort_order: i64 =
        sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), -1) + 1 FROM projects")
            .fetch_one(db)
            .await?;
    let row = ProjectRow::from_model(&model);
    sqlx::query(
        "INSERT INTO projects (id, name, description, variables_json, created_at, updated_at, sort_order)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&row.id)
    .bind(&row.name)
    .bind(&row.description)
    .bind(row.variables_json.clone())
    .bind(row.created_at.clone())
    .bind(row.updated_at.clone())
    .bind(sort_order)
    .execute(db)
    .await?;
    Ok(model)
}

pub async fn list_projects(db: &SqlitePool) -> Result<Vec<Project>> {
    let rows: Vec<ProjectRow> = sqlx::query_as(
        "SELECT id, name, description, variables_json, created_at, updated_at
         FROM projects ORDER BY sort_order ASC, created_at ASC",
    )
    .fetch_all(db)
    .await?;
    rows.into_iter().map(ProjectRow::into_model).collect()
}

/// 拖拽排序持久化：按给定 id 顺序（事务）批量重写 sort_order。
/// 任一 id 不存在则整体回滚并报错，避免前端的脏顺序污染数据库。
pub async fn update_projects_order(db: &SqlitePool, project_ids: &[Uuid]) -> Result<()> {
    let mut tx = db.begin().await?;
    for (index, id) in project_ids.iter().enumerate() {
        let result = sqlx::query("UPDATE projects SET sort_order = ? WHERE id = ?")
            .bind(index as i64)
            .bind(id.to_string())
            .execute(&mut *tx)
            .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("项目（{id}）")));
        }
    }
    tx.commit().await?;
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::memory_pool;

    #[tokio::test]
    async fn update_projects_order_persists_and_orders_list() {
        let db = memory_pool().await.expect("建库");
        let a = create_project(&db, "A", "").await.unwrap();
        let b = create_project(&db, "B", "").await.unwrap();
        let c = create_project(&db, "C", "").await.unwrap();

        // 新项目排末尾：A, B, C。
        let ids: Vec<String> = list_projects(&db)
            .await
            .unwrap()
            .into_iter()
            .map(|p| p.id.to_string())
            .collect();
        assert_eq!(ids, [a.id.to_string(), b.id.to_string(), c.id.to_string()]);

        // 拖拽成 C, A, B → 持久化并保持新序。
        update_projects_order(&db, &[c.id, a.id, b.id])
            .await
            .unwrap();
        let ids: Vec<String> = list_projects(&db)
            .await
            .unwrap()
            .into_iter()
            .map(|p| p.id.to_string())
            .collect();
        assert_eq!(ids, [c.id.to_string(), a.id.to_string(), b.id.to_string()]);

        // 不存在的 id → 整体回滚，原顺序不变。
        let missing = Uuid::new_v4();
        assert!(update_projects_order(&db, &[missing, a.id]).await.is_err());
        let ids: Vec<String> = list_projects(&db)
            .await
            .unwrap()
            .into_iter()
            .map(|p| p.id.to_string())
            .collect();
        assert_eq!(ids, [c.id.to_string(), a.id.to_string(), b.id.to_string()]);
    }
}
