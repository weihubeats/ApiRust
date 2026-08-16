//! Endpoint CRUD。

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use fox_core::model::{Endpoint, EndpointStatus, HttpMethod, RequestSpec};
use fox_core::{AppError, Result};

use super::rows::EndpointRow;

pub async fn create_endpoint(
    db: &SqlitePool,
    project_id: Uuid,
    folder_id: Option<Uuid>,
    name: &str,
) -> Result<Endpoint> {
    let now = Utc::now();
    let model = Endpoint {
        id: Uuid::new_v4(),
        project_id,
        folder_id,
        name: name.to_string(),
        method: HttpMethod::GET,
        path: "/".to_string(),
        description: String::new(),
        status: EndpointStatus::Developing,
        sort_order: 0,
        request: RequestSpec::default(),
        created_at: now,
        updated_at: now,
    };
    let row = EndpointRow::from_model(&model);
    sqlx::query(
        "INSERT INTO endpoints
         (id, project_id, folder_id, name, method, path, description, status, sort_order, request_json, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&row.id)
    .bind(&row.project_id)
    .bind(&row.folder_id)
    .bind(&row.name)
    .bind(&row.method)
    .bind(&row.path)
    .bind(&row.description)
    .bind(&row.status)
    .bind(row.sort_order)
    .bind(&row.request_json)
    .bind(row.created_at.clone())
    .bind(row.updated_at.clone())
    .execute(db)
    .await?;
    Ok(model)
}

pub async fn get_endpoint(db: &SqlitePool, endpoint_id: Uuid) -> Result<Endpoint> {
    let row: Option<EndpointRow> = sqlx::query_as(
        "SELECT id, project_id, folder_id, name, method, path, description, status,
                sort_order, request_json, created_at, updated_at
         FROM endpoints WHERE id = ?",
    )
    .bind(endpoint_id.to_string())
    .fetch_optional(db)
    .await?;
    row.map(EndpointRow::into_model)
        .transpose()?
        .ok_or_else(|| AppError::NotFound(format!("接口（{endpoint_id}）")))
}

pub async fn update_endpoint(db: &SqlitePool, endpoint: &Endpoint) -> Result<Endpoint> {
    let mut updated = endpoint.clone();
    updated.updated_at = Utc::now();
    let row = EndpointRow::from_model(&updated);
    let result = sqlx::query(
        "UPDATE endpoints SET folder_id = ?, name = ?, method = ?, path = ?, description = ?,
                status = ?, sort_order = ?, request_json = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(&row.folder_id)
    .bind(&row.name)
    .bind(&row.method)
    .bind(&row.path)
    .bind(&row.description)
    .bind(&row.status)
    .bind(row.sort_order)
    .bind(&row.request_json)
    .bind(row.updated_at.clone())
    .bind(&row.id)
    .execute(db)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("接口（{}）", endpoint.id)));
    }
    Ok(updated)
}

pub async fn delete_endpoint(db: &SqlitePool, endpoint_id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM endpoints WHERE id = ?")
        .bind(endpoint_id.to_string())
        .execute(db)
        .await?;
    Ok(())
}

pub async fn duplicate_endpoint(db: &SqlitePool, endpoint_id: Uuid) -> Result<Endpoint> {
    let source = get_endpoint(db, endpoint_id).await?;
    let now = Utc::now();
    let duplicate = Endpoint {
        id: Uuid::new_v4(),
        project_id: source.project_id,
        folder_id: source.folder_id,
        name: format!("{}（副本）", source.name),
        method: source.method,
        path: source.path,
        description: source.description,
        status: source.status,
        sort_order: source.sort_order + 1,
        request: source.request,
        created_at: now,
        updated_at: now,
    };
    let row = EndpointRow::from_model(&duplicate);
    sqlx::query(
        "INSERT INTO endpoints
         (id, project_id, folder_id, name, method, path, description, status, sort_order, request_json, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&row.id)
    .bind(&row.project_id)
    .bind(&row.folder_id)
    .bind(&row.name)
    .bind(&row.method)
    .bind(&row.path)
    .bind(&row.description)
    .bind(&row.status)
    .bind(row.sort_order)
    .bind(&row.request_json)
    .bind(row.created_at.clone())
    .bind(row.updated_at.clone())
    .execute(db)
    .await?;
    Ok(duplicate)
}

pub async fn list_endpoints(db: &SqlitePool, project_id: Uuid) -> Result<Vec<Endpoint>> {
    let rows: Vec<EndpointRow> = sqlx::query_as(
        "SELECT id, project_id, folder_id, name, method, path, description, status,
                sort_order, request_json, created_at, updated_at
         FROM endpoints WHERE project_id = ? ORDER BY sort_order, created_at",
    )
    .bind(project_id.to_string())
    .fetch_all(db)
    .await?;
    rows.into_iter().map(EndpointRow::into_model).collect()
}

/// 带 id：原样写入接口（upsert，同一 id 重复保存时更新而非报主键冲突）。
pub async fn save_endpoint(db: &SqlitePool, endpoint: &Endpoint) -> Result<()> {
    let row = EndpointRow::from_model(endpoint);
    sqlx::query(
        "INSERT INTO endpoints (id, project_id, folder_id, name, method, path, description, status, sort_order, request_json, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
            project_id = excluded.project_id,
            folder_id = excluded.folder_id,
            name = excluded.name,
            method = excluded.method,
            path = excluded.path,
            description = excluded.description,
            status = excluded.status,
            sort_order = excluded.sort_order,
            request_json = excluded.request_json,
            updated_at = excluded.updated_at",
    )
    .bind(&row.id)
    .bind(&row.project_id)
    .bind(&row.folder_id)
    .bind(&row.name)
    .bind(&row.method)
    .bind(&row.path)
    .bind(&row.description)
    .bind(&row.status)
    .bind(row.sort_order)
    .bind(&row.request_json)
    .bind(row.created_at.clone())
    .bind(row.updated_at.clone())
    .execute(db)
    .await?;
    Ok(())
}
