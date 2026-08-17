//! RustFox 端到端冒烟测试（无 UI，纯逻辑联调）。
//!
//! 覆盖四个里程碑链路：
//! 1. 创建项目 → 创建环境 → 创建接口 → 发送请求 → 查看响应 → 保存历史 → 运行测试；
//! 2. 导出 OpenAPI → 导入 → 验证数据一致；
//! 3. 启动 Mock → HTTP 请求验证 → 停止 Mock；
//! 4. 备份项目 → 恢复项目 → 验证数据一致。

use std::collections::HashMap;

use chrono::Utc;
use fox_backup::{build_backup, restore_backup, BackupFile};
use fox_core::model::{BodySpec, HttpMethod, KeyValue, RequestHistory, ResponseExample, TestRun};
use fox_core::variable::{resolve_variables_with, ResolveOptions};
use fox_http::client::send_request;
use fox_mock::server::{self, MockDefinition, MockStore};
use fox_openapi::export::export_project;
use fox_openapi::import::{import_any, ImportFormat};
use fox_storage::db::memory_pool;
use fox_storage::repository as repo;
use fox_test::runner::run_endpoint;
use serde_json::json;
use sqlx::SqlitePool;

async fn setup_pool() -> SqlitePool {
    memory_pool().await.expect("创建内存数据库")
}

// ---------- 链路 1：完整用户流程（含链路 3 的 Mock 验证） ----------

#[tokio::test]
async fn full_user_flow() {
    let db = setup_pool().await;

    // 1. 创建项目。
    let project = repo::create_project(&db, "演示项目", "冒烟测试")
        .await
        .unwrap();

    // 2. 创建环境（变量稍后通过 update_environment 写入）。
    let mut env = repo::create_environment(&db, project.id, "本地", &HashMap::new())
        .await
        .unwrap();

    // 3. 创建接口（GET /api/hello + 测试断言：状态码 200、body 包含 hello）。
    let mut ep = repo::create_endpoint(&db, project.id, None, "打招呼")
        .await
        .unwrap();
    ep.path = "/api/hello".into();
    ep.request.tests = Some(json!({
        "assertions": [
            {"name": "状态码应为 200", "type": "status", "op": "eq", "expected": 200},
            {"type": "jsonpath", "path": "$.message", "op": "contains", "expected": "hello"}
        ]
    }));
    let ep = repo::update_endpoint(&db, &ep).await.unwrap();

    // ---- 启动 Mock（链路 3 第一段）----
    let mut def = MockDefinition::from_endpoint(ep.method.as_str(), &ep.path, None);
    def.body_template = "{\"message\":\"hello from mock\",\"code\":0}".into();
    let store = MockStore::new();
    store.set_definitions(vec![def]);
    let server_mock = server::start(store).await.expect("Mock 服务启动失败");
    let base_url = server_mock.address();

    // 环境变量：base_url 指向 Mock。
    env.variables.insert("base_url".into(), base_url.clone());
    repo::update_environment(&db, &env).await.unwrap();

    // 合并变量（模拟工作区 merged_vars：项目变量 < 环境变量）。
    let mut vars = HashMap::<String, String>::new();
    vars.insert("base_url".into(), base_url);
    let url = resolve_variables_with(
        "{{base_url}}/api/hello",
        &vars,
        120,
        ResolveOptions::default(),
    );
    assert!(url.starts_with("http://127.0.0.1"), "url: {url}");

    // 4. 发送请求。
    let spec = ep.request.clone();
    let res = send_request(ep.method, &url, &spec, None)
        .await
        .expect("发送请求失败");
    assert_eq!(res.status, 200);
    assert!(
        res.body_text().contains("hello from mock"),
        "响应体: {}",
        res.body_text()
    );

    // 5. 保存历史。
    let history = RequestHistory {
        id: uuid::Uuid::new_v4(),
        project_id: project.id,
        endpoint_id: Some(ep.id),
        method: ep.method.to_string(),
        url: url.clone(),
        status: Some(res.status),
        duration_ms: Some(res.duration_ms.round() as u64),
        request_summary_json: json!({"method": ep.method.to_string(), "path": ep.path}).to_string(),
        response_summary_json: json!({
            "status": res.status,
            "duration_ms": res.duration_ms,
            "content_type": res.content_type(),
        })
        .to_string(),
        created_at: Utc::now(),
    };
    repo::save_request_history(&db, &history).await.unwrap();
    let rows = repo::list_request_histories(&db, project.id, None, 10)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1, "历史应已保存");
    assert_eq!(rows[0].url, url);

    // 6. 运行测试（配置有 2 条断言）。
    let mut runtime_vars = HashMap::<String, String>::new();
    let (result, _resp) = run_endpoint(&ep, &url, &spec, &mut runtime_vars, None).await;
    assert!(result.ok, "测试应通过: {:?}", result.request_error);
    assert_eq!(result.status, Some(200));
    assert_eq!(result.outcomes.len(), 2, "应有 2 条断言明细");

    // 运行结果入库（测试历史）。
    let run = TestRun {
        id: uuid::Uuid::new_v4(),
        project_id: project.id,
        environment_id: Some(env.id),
        name: "接口测试".into(),
        result_json: serde_json::to_string(&json!({
            "total": 1, "passed": 1, "failed": 0, "skipped": 0,
            "rows": [{"name": "打招呼", "ok": true}]
        }))
        .unwrap(),
        started_at: Utc::now() - chrono::Duration::seconds(30),
        finished_at: Some(Utc::now()),
    };
    repo::save_test_run(&db, &run).await.unwrap();
    let runs = repo::list_test_runs(&db, project.id, 20).await.unwrap();
    assert_eq!(runs.len(), 1, "测试历史应已保存");

    // ---- 停止 Mock（链路 3 收尾）----
    server_mock.stop().await;
}

// ---------- 链路 2 + 4：OpenAPI 导出/导入、备份/恢复 ----------

#[tokio::test]
async fn openapi_roundtrip_and_backup() {
    let db = setup_pool().await;
    let project = repo::create_project(&db, "接口示例", "").await.unwrap();

    let folder = repo::create_folder(&db, project.id, None, "用户")
        .await
        .unwrap();
    let mut ep = repo::create_endpoint(&db, project.id, Some(folder.id), "创建用户")
        .await
        .unwrap();
    ep.method = HttpMethod::POST;
    ep.path = "/users".into();
    ep.description = "创建用户".into();
    ep.request.params.push(KeyValue::new("debug", "1"));
    ep.request.body = BodySpec::Json {
        raw: "{\"name\":\"{{name}}\"}".into(),
    };
    let ep = repo::update_endpoint(&db, &ep).await.unwrap();

    let example = ResponseExample {
        id: uuid::Uuid::new_v4(),
        endpoint_id: ep.id,
        name: "成功".into(),
        status: 201,
        headers: HashMap::from([("x-id".into(), "7".into())]),
        body: "{\"id\":7,\"name\":\"tom\"}".into(),
        content_type: "application/json".into(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    repo::create_response_example(&db, ep.id, &example)
        .await
        .unwrap();

    // ---- 导出 → 导入 → 验证数据一致 ----
    let eps = repo::list_endpoints(&db, project.id).await.unwrap();
    let mut examples_map: HashMap<uuid::Uuid, Vec<ResponseExample>> = HashMap::new();
    for e in &eps {
        examples_map.insert(e.id, repo::list_response_examples(&db, e.id).await.unwrap());
    }
    let json = export_project(&project.name, &eps, &examples_map).expect("导出 OpenAPI");

    let (imported, format) = import_any(&json).unwrap();
    assert_eq!(
        format,
        ImportFormat::OpenApi30,
        "导出文档应识别为 OpenAPI 3.0"
    );
    assert_eq!(imported.len(), 1, "导出 1 个接口应导入 1 个");
    assert_eq!(imported[0].method, HttpMethod::POST);
    assert_eq!(imported[0].path, "/users");
    assert_eq!(imported[0].request.params[0].key, "debug");
    assert!(imported[0].examples.iter().any(|ex| ex.status == 201));

    // ---- 备份 → 恢复 → 验证数据一致 ----
    let folders = repo::list_folders(&db, project.id).await.unwrap();
    let envs = repo::list_environments(&db, project.id).await.unwrap();
    let rules = repo::list_mock_rules(&db, project.id).await.unwrap();
    let all_examples: Vec<ResponseExample> = examples_map.values().flatten().cloned().collect();

    let file = build_backup(&project, &folders, &eps, &envs, &rules, &all_examples);
    let text = file.serialize().expect("序列化备份");
    assert!(text.contains("rustfox-project-backup"));

    let parsed = BackupFile::parse(&text).expect("解析备份");
    let restored = restore_backup(&parsed);
    assert_eq!(restored.project.name, project.name);
    assert_ne!(restored.project.id, project.id, "恢复应重映射为新项目 id");
    assert_eq!(restored.folders.len(), 1);
    assert_eq!(restored.endpoints.len(), 1);
    assert_eq!(restored.response_examples.len(), 1);

    // 恢复链路可直接落库并运行（验证引用关系正确）。
    repo::save_project(&db, &restored.project).await.unwrap();
    for f in &restored.folders {
        repo::save_folder(&db, f).await.unwrap();
    }
    for e in &restored.endpoints {
        repo::save_endpoint(&db, e).await.unwrap();
    }
    let saved_ep = &restored.endpoints[0];
    assert_eq!(saved_ep.method, HttpMethod::POST);
    assert_eq!(saved_ep.path, "/users");
    assert_eq!(saved_ep.folder_id, Some(restored.folders[0].id));

    let projects = repo::list_projects(&db).await.unwrap();
    assert_eq!(projects.len(), 2, "恢复的项目应已入库");
}

// ---------- 链路 5：cURL 导入（复现 bug 报告：导入后 URL/路径/请求头丢失） ----------

/// 镜像前端 `createFromCurl` 的 URL 拆分：query → params，pathname → path。
fn split_imported_url(url: &str) -> (String, Vec<KeyValue>) {
    let (path_part, query_part) = match url.split_once('?') {
        Some((p, q)) => (p, Some(q)),
        None => (url, None),
    };
    let params = query_part
        .map(|q| {
            q.split('&')
                .filter_map(|kv| kv.split_once('='))
                .map(|(k, v)| KeyValue::new(k.to_string(), v.to_string()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    // 去掉 `scheme://host`（镜像前端 `new URL(url).pathname`）。
    let path = match path_part.split_once("://") {
        Some((_, rest)) => rest.split_once('/').map(|(_, p)| p).unwrap_or(""),
        None => path_part,
    };
    let path = path.split('/').collect::<Vec<_>>().join("/");
    let path = if path.starts_with('/') {
        path
    } else {
        format!("/{path}")
    };
    (path, params)
}

#[tokio::test]
async fn curl_import_roundtrip_keeps_url_headers_body() {
    use chrono::Utc;
    use fox_core::curl_parser::parse_curl;
    use fox_core::model::{BodySpec, Endpoint};
    use uuid::Uuid;

    let db = setup_pool().await;
    let project = repo::create_project(&db, "导入测试", "cURL 导入链路")
        .await
        .unwrap();

    // 用户报告中的命令（含续行、中文 JSON body）。
    let cmd = "curl -X POST https://jsonplaceholder.typicode.com/posts?userId=1 \
        -H 'Content-Type: application/json' -d '{\"title\":\"测试标题\",\"userId\":1}'";
    let parsed = parse_curl(cmd).expect("解析用户命令");
    assert_eq!(parsed.method, HttpMethod::POST);
    assert_eq!(
        parsed.url,
        "https://jsonplaceholder.typicode.com/posts?userId=1"
    );
    assert_eq!(parsed.headers.len(), 1);
    assert_eq!(parsed.headers[0].key, "Content-Type");

    // 镜像前端 createFromCurl：拆分 URL → path + params，组装 Endpoint。
    let (path, params) = split_imported_url(&parsed.url);
    let now = Utc::now();
    let endpoint = Endpoint {
        id: Uuid::new_v4(),
        project_id: project.id,
        folder_id: None,
        name: "posts".into(),
        method: parsed.method,
        path,
        description: String::new(),
        status: Default::default(),
        sort_order: 0,
        request: fox_core::model::RequestSpec {
            params,
            headers: parsed.headers,
            path_variables: vec![],
            auth: parsed.auth,
            body: parsed.body.unwrap_or(BodySpec::None),
            active_tab: None,
            timeout_ms: 30000,
            follow_redirects: true,
            tests: None,
        },
        created_at: now,
        updated_at: now,
    };

    repo::save_endpoint(&db, &endpoint).await.expect("落库");
    let listed = repo::list_endpoints(&db, project.id).await.unwrap();
    assert_eq!(listed.len(), 1);
    let saved = &listed[0];

    // 核心断言：路径、方法、查询参数、请求头、JSON body 全部保留。
    assert_eq!(saved.path, "/posts");
    assert_eq!(saved.method, HttpMethod::POST);
    assert_eq!(saved.request.params.len(), 1, "查询参数应保留");
    assert_eq!(saved.request.params[0].key, "userId");
    assert_eq!(saved.request.params[0].value, "1");
    assert_eq!(saved.request.headers.len(), 1);
    assert_eq!(saved.request.headers[0].key, "Content-Type");
    assert_eq!(saved.request.headers[0].value, "application/json");
    match &saved.request.body {
        BodySpec::Json { raw } => assert!(raw.contains("测试标题"), "JSON body 应保留，实际 {raw}"),
        other => panic!("期望 JSON body，实际 {other:?}"),
    }

    // 复现 bug 报告：编辑后再次保存（同 id upsert）不应报主键冲突。
    let mut updated = endpoint.clone();
    updated.path = "/posts/1".into();
    updated
        .request
        .headers
        .push(KeyValue::new("X-Custom".to_string(), "v2".to_string()));
    repo::save_endpoint(&db, &updated)
        .await
        .expect("同 id 重复保存应成功（upsert）");
    let relisted = repo::list_endpoints(&db, project.id).await.unwrap();
    assert_eq!(relisted.len(), 1, "upsert 不应产生新行");
    assert_eq!(relisted[0].path, "/posts/1", "upsert 应更新路径");
    assert_eq!(relisted[0].request.headers.len(), 2, "upsert 应更新请求头");
}
