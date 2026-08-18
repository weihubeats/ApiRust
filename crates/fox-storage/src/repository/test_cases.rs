//! TestCase（测试用例）仓储：新建 / 列表 / 删除 / 更新名称与分组 / 更新运行状态。

use sqlx::SqlitePool;
use uuid::Uuid;

use fox_core::model::{HttpMethod, KeyValue, TestCase, TestCaseStatus};
use fox_core::Result;

/// 新建测试用例。
pub async fn create_test_case(db: &SqlitePool, case: &TestCase) -> Result<TestCase> {
    sqlx::query(
        "INSERT INTO test_cases
             (id, request_id, name, category, method, url_path, params, headers,
              body_type, body_content, last_run_status, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(case.id.to_string())
    .bind(case.request_id.to_string())
    .bind(&case.name)
    .bind(&case.category)
    .bind(case.method.as_str())
    .bind(&case.url_path)
    .bind(serde_json::to_string(&case.params).map_err(fox_core::AppError::Json)?)
    .bind(serde_json::to_string(&case.headers).map_err(fox_core::AppError::Json)?)
    .bind(&case.body_type)
    .bind(&case.body_content)
    .bind(case.last_run_status.as_str())
    .bind(case.created_at.to_rfc3339())
    .execute(db)
    .await?;
    Ok(case.clone())
}

/// 列出接口的全部测试用例（按创建时间排序）。
pub async fn list_test_cases(db: &SqlitePool, request_id: Uuid) -> Result<Vec<TestCase>> {
    let rows: Vec<TestCaseRow> = sqlx::query_as(
        "SELECT id, request_id, name, category, method, url_path, params, headers,
                body_type, body_content, last_run_status, created_at
         FROM test_cases WHERE request_id = ? ORDER BY created_at",
    )
    .bind(request_id.to_string())
    .fetch_all(db)
    .await?;
    rows.into_iter().map(TestCaseRow::into_model).collect()
}

/// 删除单条测试用例。
pub async fn delete_test_case(db: &SqlitePool, case_id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM test_cases WHERE id = ?")
        .bind(case_id.to_string())
        .execute(db)
        .await?;
    Ok(())
}

/// 更新用例名称与分组（「编辑」入口）。
pub async fn update_test_case_meta(
    db: &SqlitePool,
    case_id: Uuid,
    name: &str,
    category: &str,
) -> Result<()> {
    sqlx::query("UPDATE test_cases SET name = ?, category = ? WHERE id = ?")
        .bind(name)
        .bind(category)
        .bind(case_id.to_string())
        .execute(db)
        .await?;
    Ok(())
}

/// 更新用例运行状态（直接运行 / 全部运行后回写）。
pub async fn update_test_case_status(
    db: &SqlitePool,
    case_id: Uuid,
    status: TestCaseStatus,
) -> Result<()> {
    sqlx::query("UPDATE test_cases SET last_run_status = ? WHERE id = ?")
        .bind(status.as_str())
        .bind(case_id.to_string())
        .execute(db)
        .await?;
    Ok(())
}

/// 更新用例完整请求内容（方法 / 路径 / Params / Headers / Body，抽屉「保存修改」）。
#[allow(clippy::too_many_arguments)]
pub async fn update_test_case_content(
    db: &SqlitePool,
    case_id: Uuid,
    method: HttpMethod,
    url_path: &str,
    params: &[KeyValue],
    headers: &[KeyValue],
    body_type: &str,
    body_content: &str,
) -> Result<()> {
    sqlx::query(
        "UPDATE test_cases
         SET method = ?, url_path = ?, params = ?, headers = ?, body_type = ?, body_content = ?
         WHERE id = ?",
    )
    .bind(method.as_str())
    .bind(url_path)
    .bind(serde_json::to_string(params).map_err(fox_core::AppError::Json)?)
    .bind(serde_json::to_string(headers).map_err(fox_core::AppError::Json)?)
    .bind(body_type)
    .bind(body_content)
    .bind(case_id.to_string())
    .execute(db)
    .await?;
    Ok(())
}

#[derive(sqlx::FromRow)]
struct TestCaseRow {
    id: String,
    request_id: String,
    name: String,
    category: String,
    method: String,
    url_path: String,
    params: Option<String>,
    headers: Option<String>,
    body_type: String,
    body_content: String,
    last_run_status: String,
    created_at: String,
}

impl TestCaseRow {
    fn into_model(self) -> Result<TestCase> {
        let status =
            TestCaseStatus::parse(&self.last_run_status).unwrap_or(TestCaseStatus::Untested);
        Ok(TestCase {
            id: super::rows::parse_uuid(&self.id)?,
            request_id: super::rows::parse_uuid(&self.request_id)?,
            name: self.name,
            category: self.category,
            method: self.method.parse()?,
            url_path: self.url_path,
            params: serde_json::from_str(self.params.as_deref().unwrap_or("[]"))
                .map_err(fox_core::AppError::Json)?,
            headers: serde_json::from_str(self.headers.as_deref().unwrap_or("[]"))
                .map_err(fox_core::AppError::Json)?,
            body_type: self.body_type,
            body_content: self.body_content,
            last_run_status: status,
            created_at: super::rows::parse_time(&self.created_at)?,
        })
    }
}
