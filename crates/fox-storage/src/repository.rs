//! Repository / Service 层：Project / Folder / Endpoint / Environment 的 CRUD。

use std::collections::HashMap;

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use fox_core::model::*;
use fox_core::{AppError, Result};

// ---------------------------------------------------------------------------
// 行映射（内部）
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct ProjectRow {
    id: String,
    name: String,
    description: String,
    variables_json: String,
    created_at: String,
    updated_at: String,
}

impl ProjectRow {
    fn from_model(model: &Project) -> Self {
        ProjectRow {
            id: model.id.to_string(),
            name: model.name.clone(),
            description: model.description.clone(),
            variables_json: serde_json::to_string(&model.variables).unwrap_or_else(|_| "{}".into()),
            created_at: model.created_at.to_rfc3339(),
            updated_at: model.updated_at.to_rfc3339(),
        }
    }

    fn into_model(self) -> Result<Project> {
        Ok(Project {
            id: parse_uuid(&self.id)?,
            name: self.name,
            description: self.description,
            variables: serde_json::from_str(&self.variables_json).unwrap_or_default(),
            created_at: parse_time(&self.created_at)?,
            updated_at: parse_time(&self.updated_at)?,
        })
    }
}

#[derive(sqlx::FromRow)]
struct FolderRow {
    id: String,
    project_id: String,
    parent_id: Option<String>,
    name: String,
    sort_order: i64,
    created_at: String,
    updated_at: String,
}

impl FolderRow {
    fn from_model(model: &Folder) -> FolderRow {
        FolderRow {
            id: model.id.to_string(),
            project_id: model.project_id.to_string(),
            parent_id: model.parent_id.map(|v| v.to_string()),
            name: model.name.clone(),
            sort_order: model.sort_order,
            created_at: model.created_at.to_rfc3339(),
            updated_at: model.updated_at.to_rfc3339(),
        }
    }

    fn into_model(self) -> Result<Folder> {
        Ok(Folder {
            id: parse_uuid(&self.id)?,
            project_id: parse_uuid(&self.project_id)?,
            parent_id: self.parent_id.map(|s| parse_uuid(&s)).transpose()?,
            name: self.name,
            sort_order: self.sort_order,
            created_at: parse_time(&self.created_at)?,
            updated_at: parse_time(&self.updated_at)?,
        })
    }
}

#[derive(sqlx::FromRow)]
struct EndpointRow {
    id: String,
    project_id: String,
    folder_id: Option<String>,
    name: String,
    method: String,
    path: String,
    description: String,
    status: String,
    sort_order: i64,
    request_json: String,
    created_at: String,
    updated_at: String,
}

impl EndpointRow {
    fn from_model(model: &Endpoint) -> EndpointRow {
        EndpointRow {
            id: model.id.to_string(),
            project_id: model.project_id.to_string(),
            folder_id: model.folder_id.map(|v| v.to_string()),
            name: model.name.clone(),
            method: model.method.as_str().to_string(),
            path: model.path.clone(),
            description: model.description.clone(),
            status: model.status.as_str().to_string(),
            sort_order: model.sort_order,
            request_json: serde_json::to_string(&model.request).unwrap_or_else(|_| "{}".into()),
            created_at: model.created_at.to_rfc3339(),
            updated_at: model.updated_at.to_rfc3339(),
        }
    }

    fn into_model(self) -> Result<Endpoint> {
        Ok(Endpoint {
            id: parse_uuid(&self.id)?,
            project_id: parse_uuid(&self.project_id)?,
            folder_id: self.folder_id.map(|s| parse_uuid(&s)).transpose()?,
            name: self.name,
            method: self.method.parse()?,
            path: self.path,
            description: self.description,
            status: match self.status.as_str() {
                "designing" => EndpointStatus::Designing,
                "developing" => EndpointStatus::Developing,
                "testing" => EndpointStatus::Testing,
                "released" => EndpointStatus::Released,
                "deprecated" => EndpointStatus::Deprecated,
                _ => EndpointStatus::Developing,
            },
            sort_order: self.sort_order,
            request: serde_json::from_str(&self.request_json).unwrap_or_default(),
            created_at: parse_time(&self.created_at)?,
            updated_at: parse_time(&self.updated_at)?,
        })
    }
}

#[derive(sqlx::FromRow)]
struct EnvironmentRow {
    id: String,
    project_id: String,
    name: String,
    variables_json: String,
    created_at: String,
    updated_at: String,
}

impl EnvironmentRow {
    fn from_model(model: &Environment) -> EnvironmentRow {
        EnvironmentRow {
            id: model.id.to_string(),
            project_id: model.project_id.to_string(),
            name: model.name.clone(),
            // M11：变量整体加密后落库（密钥不可用时降级明文，保证可用性）。
            variables_json: encrypt_env_json(&model.variables),
            created_at: model.created_at.to_rfc3339(),
            updated_at: model.updated_at.to_rfc3339(),
        }
    }

    fn into_model(self) -> Result<Environment> {
        Ok(Environment {
            id: parse_uuid(&self.id)?,
            project_id: parse_uuid(&self.project_id)?,
            name: self.name,
            variables: decrypt_env_json(&self.variables_json)?,
            created_at: parse_time(&self.created_at)?,
            updated_at: parse_time(&self.updated_at)?,
        })
    }
}

/// 环境变量加密（AES-256-GCM，密钥见 fox-secret）。
fn encrypt_env_json(vars: &HashMap<String, String>) -> String {
    let json = serde_json::to_string(vars).unwrap_or_else(|_| "{}".into());
    match fox_secret::ensure_master_key().and_then(|k| fox_secret::encrypt(&k, &json)) {
        Ok(cipher) => cipher,
        Err(_) => json,
    }
}

/// 环境变量解密。
///
/// 旧版本明文数据原样返回；明确加密格式但解密失败（主密钥丢失 / 更换、
/// 密文损坏）返回 `AppError::Decryption`，由 UI 层弹窗提示，
/// 避免把 base64 密文当明文解析成空变量而静默丢失。
fn decrypt_env_json(json: &str) -> Result<HashMap<String, String>> {
    let plain = fox_secret::ensure_master_key()
        .and_then(|k| fox_secret::decrypt(&k, json))
        .map_err(|e| AppError::Decryption(e.to_string()))?;
    serde_json::from_str(&plain)
        .map_err(|_| AppError::Decryption("环境变量密文已损坏，无法解析".to_string()))
}

fn parse_uuid(s: &str) -> Result<Uuid> {
    Uuid::parse_str(s).map_err(|e| AppError::Validation(format!("无效 ID：{s}（{e}）")))
}

fn parse_time(s: &str) -> Result<chrono::DateTime<Utc>> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|e| AppError::Validation(format!("无效时间：{s}（{e}）")))
}

// ---------------------------------------------------------------------------
// Project
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Folder
// ---------------------------------------------------------------------------

pub async fn create_folder(
    db: &SqlitePool,
    project_id: Uuid,
    parent_id: Option<Uuid>,
    name: &str,
) -> Result<Folder> {
    let now = Utc::now();
    let model = Folder {
        id: Uuid::new_v4(),
        project_id,
        parent_id,
        name: name.to_string(),
        sort_order: 0,
        created_at: now,
        updated_at: now,
    };
    let row = FolderRow::from_model(&model);
    sqlx::query(
        "INSERT INTO folders (id, project_id, parent_id, name, sort_order, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&row.id)
    .bind(&row.project_id)
    .bind(&row.parent_id)
    .bind(&row.name)
    .bind(row.sort_order)
    .bind(row.created_at.clone())
    .bind(row.updated_at.clone())
    .execute(db)
    .await?;
    Ok(model)
}

pub async fn list_folders(db: &SqlitePool, project_id: Uuid) -> Result<Vec<Folder>> {
    let rows: Vec<FolderRow> = sqlx::query_as(
        "SELECT id, project_id, parent_id, name, sort_order, created_at, updated_at
         FROM folders WHERE project_id = ? ORDER BY sort_order, created_at",
    )
    .bind(project_id.to_string())
    .fetch_all(db)
    .await?;
    rows.into_iter().map(FolderRow::into_model).collect()
}

pub async fn get_folder(db: &SqlitePool, folder_id: Uuid) -> Result<Folder> {
    let row: Option<FolderRow> = sqlx::query_as(
        "SELECT id, project_id, parent_id, name, sort_order, created_at, updated_at
         FROM folders WHERE id = ?",
    )
    .bind(folder_id.to_string())
    .fetch_optional(db)
    .await?;
    row.map(FolderRow::into_model)
        .transpose()?
        .ok_or_else(|| AppError::NotFound(format!("文件夹（{folder_id}）")))
}

pub async fn update_folder(db: &SqlitePool, folder: &Folder) -> Result<Folder> {
    let row = FolderRow::from_model(folder);
    sqlx::query(
        "UPDATE folders SET parent_id = ?, name = ?, sort_order = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(&row.parent_id)
    .bind(&row.name)
    .bind(row.sort_order)
    .bind(row.updated_at.clone())
    .bind(&row.id)
    .execute(db)
    .await?;
    Ok(folder.clone())
}

/// 递归收集某文件夹的整个子树（含自身）的 CTE 前缀。
///
/// folders.parent_id / endpoints.folder_id 外键均为 `ON DELETE SET NULL`，
/// 直接删除父文件夹会留下「孤儿」子文件夹与接口，因此删除时用该 CTE
/// 显式收集全部后代并级联清理。
const FOLDER_SUBTREE_SQL: &str = "WITH RECURSIVE subtree(id) AS (
    SELECT ?
    UNION ALL
    SELECT f.id FROM folders f JOIN subtree s ON f.parent_id = s.id
)";

/// 删除文件夹及其全部子孙文件夹、子孙文件夹下的接口（事务内级联）。
pub async fn delete_folder(db: &SqlitePool, folder_id: Uuid) -> Result<()> {
    let id = folder_id.to_string();
    let mut tx = db.begin().await?;

    // 先清掉子树下全部接口（外键为 SET NULL，不会自动级联删除）。
    sqlx::query(&format!(
        "{FOLDER_SUBTREE_SQL} DELETE FROM endpoints WHERE folder_id IN (SELECT id FROM subtree)"
    ))
    .bind(&id)
    .execute(&mut *tx)
    .await?;

    // 再递归删除子树全部文件夹。
    let affected = sqlx::query(&format!(
        "{FOLDER_SUBTREE_SQL} DELETE FROM folders WHERE id IN (SELECT id FROM subtree)"
    ))
    .bind(&id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    if affected.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("文件夹（{folder_id}）")));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Endpoint
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Environment
// ---------------------------------------------------------------------------

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

#[derive(sqlx::FromRow)]
struct TestRunRow {
    id: String,
    project_id: String,
    environment_id: Option<String>,
    name: String,
    result_json: String,
    started_at: String,
    finished_at: Option<String>,
}

impl TestRunRow {
    fn from_model(model: &TestRun) -> Self {
        TestRunRow {
            id: model.id.to_string(),
            project_id: model.project_id.to_string(),
            environment_id: model.environment_id.map(|e| e.to_string()),
            name: model.name.clone(),
            result_json: model.result_json.clone(),
            started_at: model.started_at.to_rfc3339(),
            finished_at: model.finished_at.map(|d| d.to_rfc3339()),
        }
    }

    fn into_model(self) -> Result<TestRun> {
        Ok(TestRun {
            id: parse_uuid(&self.id)?,
            project_id: parse_uuid(&self.project_id)?,
            environment_id: self.environment_id.as_deref().map(parse_uuid).transpose()?,
            name: self.name,
            result_json: self.result_json,
            started_at: parse_time(&self.started_at)?,
            finished_at: self.finished_at.as_deref().map(parse_time).transpose()?,
        })
    }
}

#[derive(sqlx::FromRow)]
struct HistoryRow {
    id: String,
    project_id: String,
    endpoint_id: Option<String>,
    method: String,
    url: String,
    status: Option<i64>,
    duration_ms: Option<i64>,
    request_summary_json: String,
    response_summary_json: String,
    created_at: String,
}

impl HistoryRow {
    fn from_model(model: &RequestHistory) -> Self {
        HistoryRow {
            id: model.id.to_string(),
            project_id: model.project_id.to_string(),
            endpoint_id: model.endpoint_id.map(|e| e.to_string()),
            method: model.method.clone(),
            url: model.url.clone(),
            status: model.status.map(|s| s as i64),
            duration_ms: model.duration_ms.map(|d| d as i64),
            request_summary_json: model.request_summary_json.clone(),
            response_summary_json: model.response_summary_json.clone(),
            created_at: model.created_at.to_rfc3339(),
        }
    }

    fn into_model(self) -> Result<RequestHistory> {
        Ok(RequestHistory {
            id: parse_uuid(&self.id)?,
            project_id: parse_uuid(&self.project_id)?,
            endpoint_id: match self.endpoint_id {
                Some(v) => Some(parse_uuid(&v)?),
                None => None,
            },
            method: self.method,
            url: self.url,
            status: self.status.map(|v| v as u16),
            duration_ms: self.duration_ms.map(|d| d as u64),
            request_summary_json: self.request_summary_json,
            response_summary_json: self.response_summary_json,
            created_at: parse_time(&self.created_at)?,
        })
    }
}

// ---------------------------------------------------------------------------
// Mock Rules（自定义 Mock 规则）
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct MockRuleRow {
    id: String,
    project_id: String,
    endpoint_id: Option<String>,
    name: String,
    method: String,
    path: String,
    match_query_json: String,
    match_headers_json: String,
    response_status: i64,
    response_headers_json: String,
    response_body_template: String,
    delay_ms: i64,
    enabled: i64,
    priority: i64,
    created_at: String,
    updated_at: String,
}

impl MockRuleRow {
    fn from_model(model: &MockRule) -> Self {
        MockRuleRow {
            id: model.id.to_string(),
            project_id: model.project_id.to_string(),
            endpoint_id: model.endpoint_id.map(|id| id.to_string()),
            name: model.name.clone(),
            method: model.method.as_str().to_string(),
            path: model.path.clone(),
            match_query_json: serde_json::to_string(&model.match_query)
                .unwrap_or_else(|_| "[]".into()),
            match_headers_json: serde_json::to_string(&model.match_headers)
                .unwrap_or_else(|_| "[]".into()),
            response_status: model.response_status as i64,
            response_headers_json: serde_json::to_string(&model.response_headers)
                .unwrap_or_else(|_| "{}".into()),
            response_body_template: model.response_body_template.clone(),
            delay_ms: model.delay_ms as i64,
            enabled: if model.enabled { 1 } else { 0 },
            priority: model.priority,
            created_at: model.created_at.to_rfc3339(),
            updated_at: model.updated_at.to_rfc3339(),
        }
    }

    fn into_model(self) -> Result<MockRule> {
        Ok(MockRule {
            id: parse_uuid(&self.id)?,
            project_id: parse_uuid(&self.project_id)?,
            endpoint_id: match self.endpoint_id {
                Some(id) => Some(parse_uuid(&id)?),
                None => None,
            },
            name: self.name,
            method: self.method.parse()?,
            path: self.path,
            match_query: serde_json::from_str(&self.match_query_json).unwrap_or_default(),
            match_headers: serde_json::from_str(&self.match_headers_json).unwrap_or_default(),
            response_status: self.response_status as u16,
            response_headers: serde_json::from_str(&self.response_headers_json).unwrap_or_default(),
            response_body_template: self.response_body_template,
            delay_ms: self.delay_ms as u64,
            enabled: self.enabled != 0,
            priority: self.priority,
            created_at: parse_time(&self.created_at)?,
            updated_at: parse_time(&self.updated_at)?,
        })
    }
}

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

// ---------------------------------------------------------------------------
// Request History
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// ResponseExample（响应示例）
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct ResponseExampleRow {
    id: String,
    endpoint_id: String,
    name: String,
    status: i64,
    headers_json: String,
    body: String,
    content_type: String,
    created_at: String,
    updated_at: String,
}

impl ResponseExampleRow {
    fn from_model(model: &ResponseExample) -> Self {
        ResponseExampleRow {
            id: model.id.to_string(),
            endpoint_id: model.endpoint_id.to_string(),
            name: model.name.clone(),
            status: model.status as i64,
            headers_json: serde_json::to_string(&model.headers).unwrap_or_else(|_| "{}".into()),
            body: model.body.clone(),
            content_type: model.content_type.clone(),
            created_at: model.created_at.to_rfc3339(),
            updated_at: model.updated_at.to_rfc3339(),
        }
    }

    fn into_model(self) -> Result<ResponseExample> {
        Ok(ResponseExample {
            id: parse_uuid(&self.id)?,
            endpoint_id: parse_uuid(&self.endpoint_id)?,
            name: self.name,
            status: self.status as u16,
            headers: serde_json::from_str(&self.headers_json).unwrap_or_default(),
            body: self.body,
            content_type: self.content_type,
            created_at: parse_time(&self.created_at)?,
            updated_at: parse_time(&self.updated_at)?,
        })
    }
}

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

// ---------------------------------------------------------------------------
// Settings（通用键值）
// ---------------------------------------------------------------------------

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

/// 带 id：原样写入文件夹（upsert，同一 id 重复保存时更新而非报主键冲突）。
pub async fn save_folder(db: &SqlitePool, folder: &Folder) -> Result<()> {
    let row = FolderRow::from_model(folder);
    sqlx::query(
        "INSERT INTO folders (id, project_id, parent_id, name, sort_order, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
            project_id = excluded.project_id,
            parent_id = excluded.parent_id,
            name = excluded.name,
            sort_order = excluded.sort_order,
            updated_at = excluded.updated_at",
    )
    .bind(&row.id)
    .bind(&row.project_id)
    .bind(&row.parent_id)
    .bind(&row.name)
    .bind(row.sort_order)
    .bind(row.created_at.clone())
    .bind(row.updated_at.clone())
    .execute(db)
    .await?;
    Ok(())
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

/// 带 id：原样写入 Mock 规则。
pub async fn save_mock_rule(db: &SqlitePool, rule: &MockRule) -> Result<()> {
    create_mock_rule(db, rule.project_id, rule)
        .await
        .map(|_| ())
}

/// 带 id：原样写入响应示例。
pub async fn save_response_example(db: &SqlitePool, example: &ResponseExample) -> Result<()> {
    create_response_example(db, example.endpoint_id, example)
        .await
        .map(|_| ())
}

/// 保存一次测试运行结果。
pub async fn save_test_run(db: &SqlitePool, run: &TestRun) -> Result<()> {
    let row = TestRunRow::from_model(run);
    sqlx::query(
        "INSERT INTO test_runs (id, project_id, environment_id, name, result_json, started_at, finished_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&row.id)
    .bind(&row.project_id)
    .bind(&row.environment_id)
    .bind(&row.name)
    .bind(&row.result_json)
    .bind(row.started_at.clone())
    .bind(&row.finished_at)
    .execute(db)
    .await?;
    Ok(())
}

/// 删除单条测试运行（M11 历史清理）。
pub async fn delete_test_run(db: &SqlitePool, run_id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM test_runs WHERE id = ?")
        .bind(run_id.to_string())
        .execute(db)
        .await?;
    Ok(())
}

/// 列出最近的测试运行。
pub async fn list_test_runs(db: &SqlitePool, project_id: Uuid, limit: i64) -> Result<Vec<TestRun>> {
    let rows: Vec<TestRunRow> = sqlx::query_as(
        "SELECT id, project_id, environment_id, name, result_json, started_at, finished_at
         FROM test_runs WHERE project_id = ? ORDER BY started_at DESC LIMIT ?",
    )
    .bind(project_id.to_string())
    .bind(limit)
    .fetch_all(db)
    .await?;
    rows.into_iter().map(|r| r.into_model()).collect()
}

// ---------------------------------------------------------------------------
// WebSocket 离线消息（ws_messages）
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct WsMessageRow {
    id: String,
    message_type: String,
    payload: String,
    created_at: String,
}

impl WsMessageRow {
    fn into_model(self) -> Result<WsMessageRecord> {
        Ok(WsMessageRecord {
            id: parse_uuid(&self.id)?,
            message_type: match self.message_type.as_str() {
                "text" => WsMessageType::Text,
                "binary" => WsMessageType::Binary,
                "ping" => WsMessageType::Ping,
                other => {
                    return Err(AppError::Validation(format!("无效的消息类型：{other}")));
                }
            },
            payload: self.payload,
            created_at: parse_time(&self.created_at)?,
        })
    }
}

/// 持久化一条待发消息（发送队列溢出 / 连接不可达时保活）。
pub async fn enqueue_ws_message(
    db: &SqlitePool,
    target: &str,
    message_type: WsMessageType,
    payload: &str,
) -> Result<WsMessageRecord> {
    let record = WsMessageRecord {
        id: Uuid::new_v4(),
        message_type,
        payload: payload.to_string(),
        created_at: Utc::now(),
    };
    sqlx::query(
        "INSERT INTO ws_messages (id, target, message_type, payload, created_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(record.id.to_string())
    .bind(target)
    .bind(message_type.as_str())
    .bind(&record.payload)
    .bind(record.created_at.to_rfc3339())
    .execute(db)
    .await?;
    Ok(record)
}

/// 查询指定连接的目标地址尚未发送的消息（重连成功后按序补发）。
pub async fn list_pending_ws_messages(
    db: &SqlitePool,
    target: &str,
) -> Result<Vec<WsMessageRecord>> {
    let rows: Vec<WsMessageRow> = sqlx::query_as(
        "SELECT id, message_type, payload, created_at
         FROM ws_messages WHERE target = ? ORDER BY created_at",
    )
    .bind(target)
    .fetch_all(db)
    .await?;
    rows.into_iter().map(WsMessageRow::into_model).collect()
}

/// 删除已发送的持久化消息。
pub async fn delete_ws_messages(db: &SqlitePool, ids: &[Uuid]) -> Result<()> {
    for id in ids {
        sqlx::query("DELETE FROM ws_messages WHERE id = ?")
            .bind(id.to_string())
            .execute(db)
            .await?;
    }
    Ok(())
}

/// 清理超过保留期的待发消息（过期丢弃），返回清理条数。
pub async fn purge_expired_ws_messages(
    db: &SqlitePool,
    target: &str,
    retention: chrono::Duration,
) -> Result<u64> {
    let cutoff = (Utc::now() - retention).to_rfc3339();
    let result = sqlx::query("DELETE FROM ws_messages WHERE target = ? AND created_at < ?")
        .bind(target)
        .bind(cutoff)
        .execute(db)
        .await?;
    Ok(result.rows_affected())
}
