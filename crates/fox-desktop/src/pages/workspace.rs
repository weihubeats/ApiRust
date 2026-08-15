//! 工作区页面：接口编辑器（M4）+ HTTP 调试（M5）。

use std::collections::{HashMap, HashSet};

use chrono::Utc;
use dioxus::events::eval;
use dioxus::prelude::*;
use fox_codegen::{render as render_code, GenRequest, Lang};
use fox_core::curl_parser::{parse_curl, CurlParsed};
use fox_core::model::{
    ApiKeyLocation, AuthSpec, BodySpec, Endpoint, EndpointStatus, HttpMethod, KeyValue,
    OAuth2Status, RequestHistory, RequestSpec, ResponseExample, TestRun,
};
use fox_core::util::{build_url, format_json, is_absolute_url, is_json_content_type};
use fox_core::variable::{resolve_variables_with, ResolveOptions};
use fox_core::AppError;
use fox_http::client::{describe_http_error, send_request, HttpResponseData};
use fox_openapi::markdown::export_markdown;
use fox_storage::db as storage_db;
use fox_storage::repository as repo;
use fox_test::load::{run_load, LoadConfig, LoadResult};
use fox_test::runner::{order_endpoints, run_endpoint, EndpointResult};
use serde_json::json;
use uuid::Uuid;

use crate::components::confirm_dialog::{ConfirmDialog, ConfirmInfo};
use crate::components::dropdown::Dropdown;
use crate::components::icons::{
    ClockIcon, CodeIcon, CopyIcon, PlusIcon, SaveIcon, SendIcon, XIcon,
};
use crate::components::modal::RFModal;
use crate::state::{AppState, Page};
use crate::views::empty_state::EmptyState;

/// 复制文本到剪贴板（webview 内 execCommand 兜底，兼容非安全上下文）。
fn copy_text(text: &str) {
    let quoted = serde_json::to_string(text).unwrap_or_else(|_| "\"\"".into());
    eval(&format!(
        "(function(){{var ta=document.createElement('textarea');ta.value={quoted};ta.style.position='fixed';ta.style.opacity='0';document.body.appendChild(ta);ta.focus();ta.select();try{{document.execCommand('copy');}}catch(e){{}}document.body.removeChild(ta);}})();"
    ));
}

/// 字节数人类可读格式（B / KB / MB）。
fn human_size(bytes: usize) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

/// 删除目标类型（区分测试历史 / 响应示例）。
#[derive(Clone, Copy)]
enum DeleteTarget {
    TestHistory { run_id: Uuid, project_id: Uuid },
    ResponseExample { example_id: Uuid, endpoint_id: Uuid },
}

/// 编辑器分组。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditorTab {
    Params,
    Headers,
    Body,
    Auth,
    Tests,
    Docs,
}

/// KeyValue 编辑区类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KvSection {
    Params,
    Headers,
    UrlEncoded,
}

/// Auth 字段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuthField {
    BearerToken,
    BasicUser,
    BasicPass,
    ApiKeyName,
    ApiKeyValue,
    OAuth2ClientId,
    OAuth2ClientSecret,
    OAuth2AuthUrl,
    OAuth2TokenUrl,
    OAuth2Scope,
    OAuth2RedirectUri,
}

fn kv_list(ep: &mut Endpoint, section: KvSection) -> &mut Vec<KeyValue> {
    match section {
        KvSection::Params => &mut ep.request.params,
        KvSection::Headers => &mut ep.request.headers,
        KvSection::UrlEncoded => match &mut ep.request.body {
            BodySpec::UrlEncoded { fields } => fields,
            other => {
                *other = BodySpec::UrlEncoded { fields: Vec::new() };
                match other {
                    BodySpec::UrlEncoded { fields } => fields,
                    _ => unreachable!(),
                }
            }
        },
    }
}

fn kv_row_values(ep: &Endpoint, section: KvSection, index: usize) -> Option<KeyValue> {
    match section {
        KvSection::Params => ep.request.params.get(index).cloned(),
        KvSection::Headers => ep.request.headers.get(index).cloned(),
        KvSection::UrlEncoded => match &ep.request.body {
            BodySpec::UrlEncoded { fields } => fields.get(index).cloned(),
            _ => None,
        },
    }
}

fn kv_len(ep: &Endpoint, section: KvSection) -> usize {
    match section {
        KvSection::Params => ep.request.params.len(),
        KvSection::Headers => ep.request.headers.len(),
        KvSection::UrlEncoded => match &ep.request.body {
            BodySpec::UrlEncoded { fields } => fields.len(),
            _ => 0,
        },
    }
}

/// 切换 Body 模式，尽量保留 raw 内容。
fn switch_body_mode(current: &BodySpec, mode: &str) -> BodySpec {
    let raw = match current {
        BodySpec::Json { raw } | BodySpec::Text { raw } => raw.clone(),
        _ => String::new(),
    };
    match mode {
        "json" => BodySpec::Json { raw },
        "text" => BodySpec::Text { raw },
        "urlencoded" => BodySpec::UrlEncoded {
            fields: match current {
                BodySpec::UrlEncoded { fields } => fields.clone(),
                _ => Vec::new(),
            },
        },
        _ => BodySpec::None,
    }
}

/// 切换 Body 模式并记忆/恢复可编辑内容。
/// 切到「无」/表单 再切回 JSON/文本 时，raw 与表单字段会经
/// switch_body_mode 丢弃，这里把离开前的 raw/fields 记入内存，
/// 切回时若目标内容为空则恢复，避免用户已输入的 JSON 内容凭空消失。
fn switch_body_mode_keep(
    current: &BodySpec,
    mode: &str,
    raw_keep: &mut String,
    fields_keep: &mut Vec<KeyValue>,
) -> BodySpec {
    match current {
        BodySpec::Json { raw } | BodySpec::Text { raw } => *raw_keep = raw.clone(),
        BodySpec::UrlEncoded { fields } => *fields_keep = fields.clone(),
        _ => {}
    }
    let mut next = switch_body_mode(current, mode);
    match &mut next {
        BodySpec::Json { raw } | BodySpec::Text { raw }
            if raw.is_empty() && !raw_keep.is_empty() =>
        {
            *raw = raw_keep.clone();
        }
        BodySpec::UrlEncoded { fields } if fields.is_empty() && !fields_keep.is_empty() => {
            *fields = fields_keep.clone();
        }
        _ => {}
    }
    next
}

/// 切换认证方式，尽量保留已有字段。
fn switch_auth(current: &AuthSpec, mode: &str) -> AuthSpec {
    match mode {
        "bearer" => match current {
            AuthSpec::Bearer { token } => AuthSpec::Bearer {
                token: token.clone(),
            },
            _ => AuthSpec::Bearer {
                token: String::new(),
            },
        },
        "basic" => match current {
            AuthSpec::Basic { username, password } => AuthSpec::Basic {
                username: username.clone(),
                password: password.clone(),
            },
            _ => AuthSpec::Basic {
                username: String::new(),
                password: String::new(),
            },
        },
        "apikey" => match current {
            AuthSpec::ApiKey {
                key,
                value,
                location,
            } => AuthSpec::ApiKey {
                key: key.clone(),
                value: value.clone(),
                location: *location,
            },
            _ => AuthSpec::ApiKey {
                key: String::new(),
                value: String::new(),
                location: ApiKeyLocation::Header,
            },
        },
        "oauth2" => match current {
            AuthSpec::OAuth2 {
                client_id,
                client_secret,
                auth_url,
                token_url,
                scope,
                redirect_uri,
                token,
            } => AuthSpec::OAuth2 {
                client_id: client_id.clone(),
                client_secret: client_secret.clone(),
                auth_url: auth_url.clone(),
                token_url: token_url.clone(),
                scope: scope.clone(),
                redirect_uri: redirect_uri.clone(),
                token: token.clone(),
            },
            _ => AuthSpec::OAuth2 {
                client_id: String::new(),
                client_secret: String::new(),
                auth_url: String::new(),
                token_url: String::new(),
                scope: String::new(),
                redirect_uri: fox_oauth::DEFAULT_REDIRECT_URI.to_string(),
                token: None,
            },
        },
        _ => AuthSpec::None,
    }
}

fn auth_mode(auth: &AuthSpec) -> &'static str {
    match auth {
        AuthSpec::None => "none",
        AuthSpec::Bearer { .. } => "bearer",
        AuthSpec::Basic { .. } => "basic",
        AuthSpec::ApiKey { .. } => "apikey",
        AuthSpec::OAuth2 { .. } => "oauth2",
    }
}

/// OAuth2 状态指示器：未授权 / 已授权 / 即将过期 / 已过期。
fn oauth_status_ui(ep: &Endpoint) -> Element {
    let AuthSpec::OAuth2 { token, .. } = &ep.request.auth else {
        return rsx! {};
    };
    let (class, text) = match ep.request.auth.oauth2_status() {
        Some(OAuth2Status::Unauthorized) => ("oauth-badge oauth-unauthorized", "未授权"),
        Some(OAuth2Status::Valid) => ("oauth-badge oauth-valid", "已授权"),
        Some(OAuth2Status::ExpiringSoon) => ("oauth-badge oauth-expiring", "即将过期（自动刷新）"),
        Some(OAuth2Status::Expired) => ("oauth-badge oauth-expired", "已过期（请求时自动刷新）"),
        None => return rsx! {},
    };
    let expires = token
        .as_ref()
        .map(|t| {
            format!(
                "，过期时间 {}",
                t.expires_at
                    .with_timezone(&chrono::Local)
                    .format("%Y-%m-%d %H:%M:%S")
            )
        })
        .unwrap_or_default();
    rsx! {
        div { class: "{class}", "{text}{expires}" }
    }
}

/// 「立即授权」：启动授权码流，成功后把 token 写回接口草稿并持久化。
fn authorize_oauth2(state: AppState, draft: Signal<Option<Endpoint>>) {
    let Some(ep) = draft.peek().clone() else {
        state.toast_error("没有可授权的接口");
        return;
    };
    let AuthSpec::OAuth2 { .. } = &ep.request.auth else {
        state.toast_error("认证方式不是 OAuth2");
        return;
    };
    // 配置校验。
    if ep.request.auth.oauth2_status() == Some(OAuth2Status::Unauthorized) {
        let auth = &ep.request.auth;
        if let AuthSpec::OAuth2 {
            client_id,
            auth_url,
            token_url,
            ..
        } = auth
        {
            if client_id.trim().is_empty() {
                state.toast_error("请先填写 Client ID");
                return;
            }
            if auth_url.trim().is_empty() {
                state.toast_error("请先填写授权地址 Auth URL");
                return;
            }
            if token_url.trim().is_empty() {
                state.toast_error("请先填写令牌地址 Token URL");
                return;
            }
        }
    }
    let st = state.clone();
    let mut d = draft;
    spawn(async move {
        let auth = d
            .peek()
            .clone()
            .map(|ep| ep.request.auth)
            .unwrap_or_default();
        st.toast_info("已打开浏览器，请在授权页完成授权…");
        match fox_oauth::authorize(&auth).await {
            Ok(token) => {
                tracing::info!("[OAuth2] 授权成功，access_token 已获取");
                let mut guard = d.write();
                if let Some(ep) = guard.as_mut() {
                    if let AuthSpec::OAuth2 {
                        token: slot,
                        client_id,
                        token_url,
                        ..
                    } = &mut ep.request.auth
                    {
                        *slot = Some(token.clone());
                        // 与进程内缓存保持同步（请求路径优先读缓存）。
                        fox_oauth::store_token(client_id, token_url, token.clone());
                    }
                    st.save_endpoint(ep.clone());
                }
                st.toast_success("OAuth2 授权成功");
            }
            Err(e) => {
                tracing::warn!("[OAuth2] 授权失败: {e}");
                st.toast_error(format!("OAuth2 授权失败：{e}"));
            }
        }
    });
}

/// 「清除令牌」：清空本地 token（缓存 + 草稿 + 持久化），需重新授权。
fn clear_oauth2_token(state: AppState, draft: Signal<Option<Endpoint>>) {
    let st = state.clone();
    let mut d = draft;
    spawn(async move {
        let mut guard = d.write();
        if let Some(ep) = guard.as_mut() {
            if let AuthSpec::OAuth2 {
                token,
                client_id,
                token_url,
                ..
            } = &mut ep.request.auth
            {
                *token = None;
                fox_oauth::clear_token(client_id, token_url);
            }
            st.save_endpoint(ep.clone());
        }
        st.toast_info("已清除 OAuth2 令牌，请重新授权");
    });
}

fn loc_name(loc: ApiKeyLocation) -> &'static str {
    if loc == ApiKeyLocation::Header {
        "header"
    } else {
        "query"
    }
}

fn body_raw(ep: &Endpoint) -> String {
    match &ep.request.body {
        BodySpec::Json { raw } | BodySpec::Text { raw } => raw.clone(),
        _ => String::new(),
    }
}

/// 历史记录时间展示。
fn history_time(h: &RequestHistory) -> String {
    let t = h.created_at.format("%m-%d %H:%M:%S");
    match h.duration_ms {
        Some(d) => format!("{t} · {d} ms"),
        None => t.to_string(),
    }
}

/// 加载历史列表（普通函数，避免闭包捕获问题）。
fn load_history_list(st: AppState, histories: Signal<Vec<RequestHistory>>) {
    let Some(project_id) = *st.current_project_id.peek() else {
        return;
    };
    let db = st.services.db.clone();
    let mut ht = histories;
    spawn(async move {
        match repo::list_request_histories(&db, project_id, 50).await {
            Ok(list) => ht.set(list),
            Err(e) => st.toast_error(format!("加载历史失败：{}", e.user_message())),
        }
    });
}

/// 历史条目。
fn history_item(h: &RequestHistory, selected: Signal<Option<RequestHistory>>) -> Element {
    let hh = h.clone();
    let method_cls = h.method.to_lowercase();
    let status = h.status;
    rsx! {
        div { class: "history-item",
            onclick: move |_| {
                let mut sel = selected;
                sel.set(Some(hh.clone()));
            },
            div { class: "history-meta",
                span { class: "rf-method rf-method-chip rf-method-chip-{method_cls}", "{h.method}" }
                span { class: "url", "{h.url}" }
                if let Some(status) = status {
                    span { class: if status < 400 { "status-ok" } else { "status-err" }, "{status}" }
                }
            }
            div { class: "history-time", "{history_time(h)}" }
        }
    }
}

/// 高亮 token 节点
#[derive(Clone)]
struct HiNode {
    cls: &'static str,
    text: String,
}

/// JSON 词法分析器：输出带颜色类的 token 列表。
fn tokenize_json(text: &str) -> Vec<HiNode> {
    let mut out = Vec::new();
    let mut chars = text.char_indices().peekable();

    while let Some((_, c)) = chars.next() {
        match c {
            '"' => {
                // 读字符串（含转义）
                let mut s = String::from('"');
                loop {
                    match chars.next() {
                        Some((_, '\\')) => {
                            s.push('\\');
                            if let Some((_, esc)) = chars.next() {
                                s.push(esc);
                            }
                        }
                        Some((_, '"')) => {
                            s.push('"');
                            break;
                        }
                        Some((_, ch)) => s.push(ch),
                        None => break,
                    }
                }
                out.push(HiNode {
                    cls: "hl-s",
                    text: s,
                });
            }
            '0'..='9' | '-' => {
                let mut s = String::from(c);
                while let Some((_, ch)) = chars.peek() {
                    if ch.is_ascii_digit()
                        || *ch == '.'
                        || *ch == '-'
                        || *ch == '+'
                        || *ch == 'e'
                        || *ch == 'E'
                    {
                        s.push(*ch);
                        chars.next();
                    } else {
                        break;
                    }
                }
                out.push(HiNode {
                    cls: "hl-n",
                    text: s,
                });
            }
            't' | 'f' | 'n' => {
                let mut word = String::from(c);
                for (_, ch) in chars.by_ref() {
                    if ch.is_alphabetic() {
                        word.push(ch);
                    } else {
                        break;
                    }
                }
                if matches!(word.as_str(), "true" | "false" | "null") {
                    out.push(HiNode {
                        cls: "hl-b",
                        text: word,
                    });
                } else {
                    out.push(HiNode {
                        cls: "",
                        text: word,
                    });
                }
            }
            '{' | '}' | '[' | ']' | ':' | ',' => {
                out.push(HiNode {
                    cls: "hl-p",
                    text: c.to_string(),
                });
            }
            ' ' | '\t' | '\n' | '\r' => {
                out.push(HiNode {
                    cls: "",
                    text: c.to_string(),
                });
            }
            _ => {
                out.push(HiNode {
                    cls: "",
                    text: c.to_string(),
                });
            }
        }
    }

    // 第二遍：把紧跟在冒号前的字符串标为 key
    for i in 0..out.len() {
        if out[i].cls == "hl-s" {
            let mut j = i + 1;
            while j < out.len() && out[j].cls.is_empty() {
                j += 1;
            }
            if j < out.len() && out[j].text == ":" {
                out[i].cls = "hl-k";
            }
        }
    }

    out
}

/// 高亮 JSON 文本组件（供 pre 包裹使用）
fn highlight_json(text: String) -> Element {
    if text.is_empty() {
        return rsx! {};
    }
    let nodes = tokenize_json(&text);
    rsx! {
        for node in nodes {
            span { class: "{node.cls}", "{node.text}" }
        }
    }
}

/// 历史摘要美化展示。
fn prettify_summary(raw: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(raw) {
        Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_else(|_| raw.to_string()),
        Err(_) => raw.to_string(),
    }
}

/// 响应视图（供 UI 展示）。
#[derive(Debug, Clone, PartialEq)]
struct ResponseView {
    status: u16,
    duration_ms: u64,
    size_bytes: usize,
    truncated: bool,
    headers: Vec<(String, String)>,
    content_type: String,
    /// Pretty 格式化后的正文（JSON 时格式化，其余同 raw）。
    body: String,
    /// 原始正文（Pretty/Raw 切换用）。
    raw_body: String,
}

/// 测试结果视图（供 UI 展示）。
#[derive(Debug, Clone, PartialEq)]
struct TestRowView {
    name: String,
    method: String,
    path: String,
    ok: bool,
    skipped: bool,
    status: Option<u16>,
    duration_ms: Option<u64>,
    error: Option<String>,
    failures: Vec<String>,
}

/// 一次测试运行的展示视图。
#[derive(Debug, Clone, PartialEq)]
struct TestRunView {
    name: String,
    total: usize,
    passed: usize,
    failed: usize,
    skipped: usize,
    rows: Vec<TestRowView>,
}

/// 加载某接口的响应示例（M10）。
fn load_examples(st: AppState, endpoint_id: Option<Uuid>, examples: Signal<Vec<ResponseExample>>) {
    let Some(id) = endpoint_id else {
        let mut ex = examples;
        ex.set(Vec::new());
        return;
    };
    let db = st.services.db.clone();
    let mut ex = examples;
    spawn(async move {
        match repo::list_response_examples(&db, id).await {
            Ok(list) => ex.set(list),
            Err(e) => st.toast_error(format!("加载响应示例失败：{}", e.user_message())),
        }
    });
}

/// 删除一条响应示例（M10）。
fn delete_example(
    st: AppState,
    example_id: Uuid,
    endpoint_id: Uuid,
    examples: Signal<Vec<ResponseExample>>,
) {
    let db = st.services.db.clone();
    let ex = examples;
    spawn(async move {
        match repo::delete_response_example(&db, example_id).await {
            Ok(()) => {
                load_examples(st.clone(), Some(endpoint_id), ex);
                st.toast_success("响应示例已删除");
            }
            Err(e) => st.toast_error(format!("删除响应示例失败：{}", e.user_message())),
        }
    });
}

/// 导出整个项目 Markdown 文档（M10）。
fn export_markdown_file(st: AppState) {
    let Some(project_id) = *st.current_project_id.peek() else {
        st.toast_error("未选择项目");
        return;
    };
    let name = st
        .projects
        .read()
        .iter()
        .find(|p| p.id == project_id)
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "项目".to_string());
    let endpoints = st.endpoints.read().clone();
    let db = st.services.db.clone();
    spawn(async move {
        let mut examples_by_ep: HashMap<Uuid, Vec<ResponseExample>> = HashMap::new();
        for ep in &endpoints {
            match repo::list_response_examples(&db, ep.id).await {
                Ok(list) => {
                    examples_by_ep.insert(ep.id, list);
                }
                Err(e) => {
                    st.toast_error(format!("加载响应示例失败：{}", e.user_message()));
                    return;
                }
            }
        }
        let md = export_markdown(&name, &endpoints, &examples_by_ep);
        let dir = storage_db::data_dir().join("exports");
        if std::fs::create_dir_all(&dir).is_err() {
            st.toast_error("创建导出目录失败");
            return;
        }
        let filename = format!(
            "{}_{}.md",
            name.trim().replace(['/', ':', '\\'], "_").replace(' ', "_"),
            Utc::now().format("%Y%m%d_%H%M%S")
        );
        let path = dir.join(filename);
        match std::fs::write(&path, md) {
            Ok(()) => st.toast_success(format!("Markdown 已导出：{}", path.display())),
            Err(e) => st.toast_error(format!("Markdown 导出失败：{e}")),
        }
    });
}

/// 把当前响应保存为响应示例（M10）。
fn save_response_example(
    st: AppState,
    endpoint_id: Uuid,
    view: ResponseView,
    examples: Signal<Vec<ResponseExample>>,
) {
    let db = st.services.db.clone();
    let ex = examples;
    spawn(async move {
        let now = Utc::now();
        let example = ResponseExample {
            id: Uuid::new_v4(),
            endpoint_id,
            name: format!("示例 @ {}", now.format("%H:%M:%S")),
            status: view.status,
            headers: view
                .headers
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            body: view.body.clone(),
            content_type: view.content_type.clone(),
            created_at: now,
            updated_at: now,
        };
        match repo::create_response_example(&db, endpoint_id, &example).await {
            Ok(_) => {
                load_examples(st.clone(), Some(endpoint_id), ex);
                st.toast_success("已保存为响应示例（Docs Tab 可查看）");
            }
            Err(e) => st.toast_error(format!("保存响应示例失败：{}", e.user_message())),
        }
    });
}

/// 把 runner 结果转成 UI 视图。
fn to_test_row(r: &EndpointResult) -> TestRowView {
    let skipped = r
        .request_error
        .as_deref()
        .is_some_and(|e| e.contains("无测试配置"));
    TestRowView {
        name: r.endpoint_name.clone(),
        method: r.method.clone(),
        path: r.path.clone(),
        ok: r.ok,
        skipped,
        status: r.status,
        duration_ms: r.duration_ms,
        error: r.request_error.clone(),
        failures: r
            .outcomes
            .iter()
            .filter(|o| !o.passed)
            .map(|o| match &o.reason {
                Some(r) => format!("{}：{r}", o.description),
                None => o.description.clone(),
            })
            .collect(),
    }
}

/// 一条测试历史（运行记录 + 解析出的视图）。
#[derive(Clone, PartialEq)]
struct HistoryEntry {
    run: TestRun,
    view: TestRunView,
}

/// 解析 result_json 为展示视图（容错缺失字段）。
fn parse_run_view(run: &TestRun) -> TestRunView {
    let fallback = TestRunView {
        name: run.name.clone(),
        total: 0,
        passed: 0,
        failed: 0,
        skipped: 0,
        rows: Vec::new(),
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&run.result_json) else {
        return fallback;
    };
    let num = |k: &str| v.get(k).and_then(|x| x.as_u64()).unwrap_or(0) as usize;
    let rows = v
        .get("rows")
        .and_then(|x| x.as_array())
        .map(|arr| {
            arr.iter()
                .map(|r| TestRowView {
                    name: r
                        .get("name")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                    method: r
                        .get("method")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                    path: r
                        .get("path")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                    ok: r.get("ok").and_then(|x| x.as_bool()).unwrap_or(false),
                    skipped: r.get("skipped").and_then(|x| x.as_bool()).unwrap_or(false),
                    status: r.get("status").and_then(|x| x.as_u64()).map(|x| x as u16),
                    duration_ms: r.get("duration_ms").and_then(|x| x.as_u64()),
                    error: r
                        .get("error")
                        .and_then(|x| x.as_str())
                        .map(|x| x.to_string()),
                    failures: r
                        .get("failures")
                        .and_then(|x| x.as_array())
                        .map(|f| {
                            f.iter()
                                .filter_map(|x| x.as_str())
                                .map(|x| x.to_string())
                                .collect()
                        })
                        .unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default();
    TestRunView {
        name: v
            .get("name")
            .and_then(|x| x.as_str())
            .unwrap_or(&run.name)
            .to_string(),
        total: num("total"),
        passed: num("passed"),
        failed: num("failed"),
        skipped: num("skipped"),
        rows,
    }
}

/// 加载测试历史（最近 20 次）。
fn load_test_history(st: AppState, project_id: Option<Uuid>, history: Signal<Vec<HistoryEntry>>) {
    let Some(pid) = project_id else {
        let mut h = history;
        h.set(Vec::new());
        return;
    };
    let db = st.services.db.clone();
    let mut h = history;
    spawn(async move {
        match repo::list_test_runs(&db, pid, 20).await {
            Ok(runs) => {
                let entries = runs
                    .into_iter()
                    .map(|run| {
                        let view = parse_run_view(&run);
                        HistoryEntry { run, view }
                    })
                    .collect();
                h.set(entries);
            }
            Err(e) => st.toast_error(format!("加载测试历史失败：{}", e.user_message())),
        }
    });
}

/// 删除一条测试历史并刷新。
fn delete_test_history_entry(
    st: AppState,
    run_id: Uuid,
    project_id: Uuid,
    history: Signal<Vec<HistoryEntry>>,
) {
    let db = st.services.db.clone();
    let h = history;
    spawn(async move {
        match repo::delete_test_run(&db, run_id).await {
            Ok(()) => {
                load_test_history(st.clone(), Some(project_id), h);
                st.toast_success("测试历史已删除");
            }
            Err(e) => st.toast_error(format!("删除失败：{}", e.user_message())),
        }
    });
}

/// 执行某次测试（按目录排序、共享运行时变量），完成后展示结果并入库。
fn run_tests(
    st: AppState,
    endpoints: Vec<Endpoint>,
    run_name: String,
    mut test_result: Signal<Option<TestRunView>>,
    mut running: Signal<bool>,
) {
    running.set(true);
    spawn(async move {
        let Some(project_id) = *st.current_project_id.peek() else {
            st.toast_error("未选择项目");
            running.set(false);
            return;
        };
        tracing::info!("用户运行测试 count={}", endpoints.len());
        st.record_step(format!("运行测试（{} 个接口）", endpoints.len()));
        let folder_ids: HashMap<Uuid, i64> = st
            .folders
            .read()
            .iter()
            .map(|f| (f.id, f.sort_order))
            .collect();
        let ordered = order_endpoints(&endpoints, &folder_ids);

        let mut vars = merged_vars(&st, project_id);
        let mut rows: Vec<TestRowView> = Vec::with_capacity(ordered.len());
        for ep in ordered {
            let (url, spec) = render_request(ep, &vars);
            if !url.starts_with("http://") && !url.starts_with("https://") {
                rows.push(TestRowView {
                    name: ep.name.clone(),
                    method: ep.method.to_string(),
                    path: ep.path.clone(),
                    ok: false,
                    skipped: false,
                    status: None,
                    duration_ms: None,
                    error: Some(format!("URL 无效：{url}")),
                    failures: Vec::new(),
                });
                continue;
            }
            let (r, _) = run_endpoint(ep, &url, &spec, &mut vars, None).await;
            rows.push(to_test_row(&r));
        }

        let skipped = rows.iter().filter(|r| r.skipped).count();
        let failed = rows.iter().filter(|r| !r.ok && !r.skipped).count();
        let passed = rows.len() - skipped - failed;
        let view = TestRunView {
            name: run_name.clone(),
            total: rows.len(),
            passed,
            failed,
            skipped,
            rows,
        };
        test_result.set(Some(view.clone()));
        running.set(false);

        // 入库。
        let db = st.services.db.clone();
        let run_json = serde_json::json!({
            "name": run_name,
            "total": view.total,
            "passed": view.passed,
            "failed": view.failed,
            "skipped": view.skipped,
            "rows": view.rows.iter().map(|r| serde_json::json!({
                "name": r.name, "method": r.method, "path": r.path, "ok": r.ok,
                "skipped": r.skipped, "status": r.status, "duration_ms": r.duration_ms,
                "error": r.error, "failures": r.failures
            })).collect::<Vec<_>>(),
        });
        let count = view.total;
        let passed2 = view.passed;
        let failed2 = view.failed;
        let skipped2 = view.skipped;
        let run = TestRun {
            id: Uuid::new_v4(),
            project_id,
            environment_id: *st.current_environment_id.peek(),
            name: run_name.clone(),
            result_json: run_json.to_string(),
            started_at: Utc::now() - chrono::Duration::seconds(60),
            finished_at: Some(Utc::now()),
        };
        if let Err(e) = repo::save_test_run(&db, &run).await {
            st.toast_error(format!("保存测试结果失败：{}", e.user_message()));
        } else {
            st.toast_success(format!(
                "测试完成：{passed2} 通过 / {failed2} 失败 / {skipped2} 跳过（共 {count}）"
            ));
        }
    });
}

/// M14：压测（并发基准）。结果展示在压测面板，并作为单行测试入库。
fn run_load_benchmark(
    st: AppState,
    ep: Endpoint,
    concurrency: usize,
    total: usize,
    mut load_result: Signal<Option<LoadResult>>,
    mut running: Signal<bool>,
    mut test_result: Signal<Option<TestRunView>>,
) {
    running.set(true);
    spawn(async move {
        let Some(project_id) = *st.current_project_id.peek() else {
            st.toast_error("未选择项目");
            running.set(false);
            return;
        };
        let vars = merged_vars(&st, project_id);
        let (url, spec) = render_request(&ep, &vars);
        if !url.starts_with("http://") && !url.starts_with("https://") {
            st.toast_error(format!("URL 无效：{url}"));
            running.set(false);
            return;
        }
        let cfg = LoadConfig { concurrency, total };
        st.record_step(format!("开始压测（{} 并发 × {} 次）", concurrency, total));
        let result = run_load(ep.method, &url, &spec, &cfg).await;
        load_result.set(Some(result.clone()));
        running.set(false);

        let db = st.services.db.clone();
        let run_json = serde_json::json!({
            "kind": "load",
            "concurrency": concurrency,
            "total": result.total,
            "ok": result.ok,
            "failed": result.failed,
            "total_ms": result.total_ms,
            "avg_ms": result.avg_ms,
            "p50_ms": result.p50_ms,
            "p90_ms": result.p90_ms,
            "p99_ms": result.p99_ms,
            "rps": result.rps,
            "errors": result.errors,
        });
        let ok = result.failed == 0;
        let row = TestRowView {
            name: format!("压测 {} {}", ep.method, ep.path),
            method: ep.method.to_string(),
            path: url.clone(),
            ok,
            skipped: false,
            status: None,
            duration_ms: Some(result.total_ms),
            error: if ok {
                None
            } else {
                Some(format!("{} 次失败", result.failed))
            },
            failures: result.errors.clone(),
        };
        let view = TestRunView {
            name: format!("压测 {concurrency} 并发 × {total} 次"),
            total: 1,
            passed: if ok { 1 } else { 0 },
            failed: if ok { 0 } else { 1 },
            skipped: 0,
            rows: vec![row],
        };
        test_result.set(Some(view.clone()));
        let run = TestRun {
            id: Uuid::new_v4(),
            project_id,
            environment_id: *st.current_environment_id.peek(),
            name: format!("压测 {} {}", ep.method, ep.path),
            result_json: run_json.to_string(),
            started_at: Utc::now() - chrono::Duration::seconds(60),
            finished_at: Some(Utc::now()),
        };
        if let Err(e) = repo::save_test_run(&db, &run).await {
            st.toast_error(format!("保存压测结果失败：{}", e.user_message()));
        } else {
            st.toast_success(format!(
                "压测完成：{}/{} 成功（QPS {:.1}）",
                result.ok, result.total, result.rps
            ));
        }
    });
}

fn resolve_text(input: &str, vars: &HashMap<String, String>) -> String {
    resolve_variables_with(input, vars, 120, ResolveOptions::default())
}

/// 合并变量：项目变量 < 环境变量。
fn merged_vars(state: &AppState, project_id: Uuid) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Some(p) = state.projects.read().iter().find(|p| p.id == project_id) {
        map.extend(p.variables.clone());
    }
    if let Some(env_id) = *state.current_environment_id.peek() {
        if let Some(env) = state.environments.read().iter().find(|e| e.id == env_id) {
            map.extend(env.variables.clone());
        }
    }
    map
}

/// M15：空白接口（新标签草稿）。
fn blank_endpoint(project_id: Uuid) -> Endpoint {
    let now = Utc::now();
    Endpoint {
        id: Uuid::new_v4(),
        project_id,
        folder_id: None,
        name: "新建接口".into(),
        method: HttpMethod::GET,
        path: "/api/new".into(),
        description: String::new(),
        status: EndpointStatus::Developing,
        sort_order: 0,
        request: RequestSpec::default(),
        created_at: now,
        updated_at: now,
    }
}

/// M17：把 cURL 解析结果应用到一个接口草稿（方法、路径、请求头、Body、认证）。
fn apply_curl(ep: &mut Endpoint, parsed: &CurlParsed) {
    ep.method = parsed.method;
    ep.path = parsed.url.clone();
    ep.request.headers = parsed.headers.clone();
    ep.request.auth = parsed.auth.clone();
    if let Some(body) = &parsed.body {
        ep.request.body = body.clone();
    }
    if ep.name.trim().is_empty() || ep.name.trim() == "新建接口" {
        let last = parsed
            .url
            .trim_end_matches('/')
            .rsplit('/')
            .find(|s| !s.is_empty());
        ep.name = last.unwrap_or("导入接口").to_string();
    }
}

/// M15：关闭标签（活动标签关闭后切换到最后一个，未保存修改丢弃并提示）。
fn close_tab_impl(
    st: AppState,
    id: Uuid,
    mut tabs: Signal<Vec<Uuid>>,
    mut tds: Signal<HashMap<Uuid, Endpoint>>,
    mut dirty: Signal<HashSet<Uuid>>,
    mut active: Signal<Option<Uuid>>,
    mut draft: Signal<Option<Endpoint>>,
) {
    if dirty.peek().contains(&id) {
        st.toast_info("已关闭：该标签存在未保存的修改");
    }
    let was_active = match *active.peek() {
        Some(cur) => cur == id,
        None => false,
    };
    tabs.write().retain(|x| *x != id);
    tds.write().remove(&id);
    dirty.write().remove(&id);
    if was_active {
        match tabs.peek().last().copied() {
            Some(nid) => {
                let ep = tds.peek().get(&nid).cloned();
                match ep {
                    Some(ep) => draft.set(Some(ep)),
                    None => draft.set(None),
                }
                active.set(Some(nid));
            }
            None => {
                active.set(None);
                draft.set(None);
            }
        }
    }
}

/// 渲染 URL 与请求规格（变量替换 + 路径变量 + base_url）。
fn render_request(ep: &Endpoint, vars: &HashMap<String, String>) -> (String, RequestSpec) {
    let path_vars: HashMap<String, String> = ep
        .request
        .path_variables
        .iter()
        .filter(|kv| kv.enabled)
        .map(|kv| (kv.key.clone(), resolve_text(&kv.value, vars)))
        .collect();
    let rendered_path = resolve_text(&ep.path, vars);
    let base = vars.get("base_url").cloned().unwrap_or_default();
    let url = build_url(
        if base.is_empty() { None } else { Some(&base) },
        &rendered_path,
        &path_vars,
    );

    let mut spec = ep.request.clone();
    for kv in &mut spec.params {
        if kv.enabled {
            kv.key = resolve_text(&kv.key, vars);
            kv.value = resolve_text(&kv.value, vars);
        }
    }
    for kv in &mut spec.headers {
        if kv.enabled {
            kv.key = resolve_text(&kv.key, vars);
            kv.value = resolve_text(&kv.value, vars);
        }
    }
    for kv in &mut spec.path_variables {
        if kv.enabled {
            kv.key = resolve_text(&kv.key, vars);
            kv.value = resolve_text(&kv.value, vars);
        }
    }
    match &mut spec.body {
        BodySpec::Json { raw } | BodySpec::Text { raw } => {
            *raw = resolve_text(raw, vars);
        }
        BodySpec::UrlEncoded { fields } => {
            for kv in fields {
                if kv.enabled {
                    kv.key = resolve_text(&kv.key, vars);
                    kv.value = resolve_text(&kv.value, vars);
                }
            }
        }
        _ => {}
    }
    match &mut spec.auth {
        AuthSpec::Bearer { token } => *token = resolve_text(token, vars),
        AuthSpec::Basic { username, password } => {
            *username = resolve_text(username, vars);
            *password = resolve_text(password, vars);
        }
        AuthSpec::ApiKey { key, value, .. } => {
            *key = resolve_text(key, vars);
            *value = resolve_text(value, vars);
        }
        AuthSpec::OAuth2 {
            client_id,
            client_secret,
            auth_url,
            token_url,
            scope,
            redirect_uri,
            token,
        } => {
            // OAuth2 配置字段支持变量插值。
            *client_id = resolve_text(client_id, vars);
            *client_secret = resolve_text(client_secret, vars);
            *auth_url = resolve_text(auth_url, vars);
            *token_url = resolve_text(token_url, vars);
            *scope = resolve_text(scope, vars);
            *redirect_uri = resolve_text(redirect_uri, vars);
            // 进程内缓存中的新令牌优先（自动刷新后保持最新）。
            if let Some(cached) = fox_oauth::cached_token(client_id, token_url) {
                *token = Some(cached);
            }
        }
        AuthSpec::None => {}
    }
    (url, spec)
}

/// M13：生成客户端代码（变量替换后的完整请求）。
pub(crate) fn build_codegen_code(st: &AppState, ep: &Endpoint, lang: Lang) -> Option<String> {
    let project_id = (*st.current_project_id.peek())?;
    let vars = merged_vars(st, project_id);
    let (url, spec) = render_request(ep, &vars);
    let req = GenRequest {
        method: &ep.method,
        url: &url,
        headers: &spec.headers,
        body: &spec.body,
        auth: &spec.auth,
    };
    Some(render_code(lang, &req))
}

/// 忽略 updated_at 比较接口是否一致：数据库保存时会刷新 updated_at，
/// 而草稿里保留旧值，若全字段比较则保存后脏标记永不消失。
fn eq_ignoring_updated_at(a: &Endpoint, b: &Endpoint) -> bool {
    let mut a = a.clone();
    let b = b.clone();
    a.updated_at = b.updated_at;
    a == b
}
/// 历史摘要保存。
fn build_history(
    ep: &Endpoint,
    project_id: Uuid,
    url: &str,
    data: &HttpResponseData,
) -> RequestHistory {
    let body_text = data.body_text();
    let body_preview: String = body_text.chars().take(2000).collect();
    RequestHistory {
        id: Uuid::new_v4(),
        project_id,
        endpoint_id: Some(ep.id),
        method: ep.method.to_string(),
        url: url.to_string(),
        status: Some(data.status),
        duration_ms: Some(data.duration_ms),
        request_summary_json: json!({
            "path": ep.path,
            "method": ep.method.to_string(),
            "headers": ep.request.headers.iter().filter(|kv| kv.enabled)
                .map(|kv| [kv.key.clone(), kv.value.clone()]).collect::<Vec<_>>(),
        })
        .to_string(),
        response_summary_json: json!({
            "status": data.status,
            "duration_ms": data.duration_ms,
            "size_bytes": data.size_bytes,
            "truncated": data.truncated,
            "content_type": data.content_type(),
            "body": body_preview,
        })
        .to_string(),
        created_at: Utc::now(),
    }
}

/// 把响应转成展示视图（JSON 自动格式化）。
fn to_response_view(data: HttpResponseData) -> Option<ResponseView> {
    let content_type = data.content_type();
    let raw = data.body_text();
    let body = if is_json_content_type(&content_type) {
        format_json(&raw).unwrap_or(raw.clone())
    } else {
        raw.clone()
    };
    Some(ResponseView {
        status: data.status,
        duration_ms: data.duration_ms,
        size_bytes: data.size_bytes,
        truncated: data.truncated,
        headers: data.headers,
        content_type,
        body,
        raw_body: raw,
    })
}

fn auth_value(auth: &AuthSpec, field: AuthField) -> String {
    match (auth, field) {
        (AuthSpec::Bearer { token }, AuthField::BearerToken) => token.clone(),
        (AuthSpec::Basic { username, .. }, AuthField::BasicUser) => username.clone(),
        (AuthSpec::Basic { password, .. }, AuthField::BasicPass) => password.clone(),
        (AuthSpec::ApiKey { key, .. }, AuthField::ApiKeyName) => key.clone(),
        (AuthSpec::ApiKey { value, .. }, AuthField::ApiKeyValue) => value.clone(),
        (AuthSpec::OAuth2 { client_id, .. }, AuthField::OAuth2ClientId) => client_id.clone(),
        (AuthSpec::OAuth2 { client_secret, .. }, AuthField::OAuth2ClientSecret) => {
            client_secret.clone()
        }
        (AuthSpec::OAuth2 { auth_url, .. }, AuthField::OAuth2AuthUrl) => auth_url.clone(),
        (AuthSpec::OAuth2 { token_url, .. }, AuthField::OAuth2TokenUrl) => token_url.clone(),
        (AuthSpec::OAuth2 { scope, .. }, AuthField::OAuth2Scope) => scope.clone(),
        (AuthSpec::OAuth2 { redirect_uri, .. }, AuthField::OAuth2RedirectUri) => {
            redirect_uri.clone()
        }
        _ => String::new(),
    }
}

fn set_auth_value(auth: &mut AuthSpec, field: AuthField, v: String) {
    match (auth, field) {
        (AuthSpec::Bearer { token }, AuthField::BearerToken) => *token = v,
        (AuthSpec::Basic { username, .. }, AuthField::BasicUser) => *username = v,
        (AuthSpec::Basic { password, .. }, AuthField::BasicPass) => *password = v,
        (AuthSpec::ApiKey { key, .. }, AuthField::ApiKeyName) => *key = v,
        (AuthSpec::ApiKey { value, .. }, AuthField::ApiKeyValue) => *value = v,
        (AuthSpec::OAuth2 { client_id, .. }, AuthField::OAuth2ClientId) => *client_id = v,
        (AuthSpec::OAuth2 { client_secret, .. }, AuthField::OAuth2ClientSecret) => {
            *client_secret = v
        }
        (AuthSpec::OAuth2 { auth_url, .. }, AuthField::OAuth2AuthUrl) => *auth_url = v,
        (AuthSpec::OAuth2 { token_url, .. }, AuthField::OAuth2TokenUrl) => *token_url = v,
        (AuthSpec::OAuth2 { scope, .. }, AuthField::OAuth2Scope) => *scope = v,
        (AuthSpec::OAuth2 { redirect_uri, .. }, AuthField::OAuth2RedirectUri) => *redirect_uri = v,
        _ => {}
    }
}

/// 渲染一个 KeyValue 编辑行。
fn kv_row(draft: Signal<Option<Endpoint>>, section: KvSection, index: usize) -> Element {
    let row = match draft
        .peek()
        .as_ref()
        .and_then(|ep| kv_row_values(ep, section, index))
    {
        Some(row) => row,
        None => return rsx! {},
    };

    let key = row.key.clone();
    let value = row.value.clone();
    let desc = row.description.clone();
    let enabled = row.enabled;

    rsx! {
        tr {
            td {
                input {
                    r#type: "checkbox",
                    class: "rf-check",
                    checked: enabled,
                    onchange: move |e| {
                        let c = e.data().checked();
                        let mut d = draft;
                        let mut guard = d.write();
                        if let Some(ep) = guard.as_mut() {
                            if let Some(kv) = kv_list(ep, section).get_mut(index) {
                                kv.enabled = c;
                            }
                        }
                    },
                }
            }
            td {
                input {
                    class: "rf-input rf-kv-input",
                    value: "{key}",
                    oninput: move |e| {
                        let v = e.data().value();
                        let mut d = draft;
                        let mut guard = d.write();
                        if let Some(ep) = guard.as_mut() {
                            if let Some(kv) = kv_list(ep, section).get_mut(index) {
                                kv.key = v;
                            }
                        }
                    },
                }
            }
            td {
                input {
                    class: "rf-input rf-kv-input",
                    value: "{value}",
                    oninput: move |e| {
                        let v = e.data().value();
                        let mut d = draft;
                        let mut guard = d.write();
                        if let Some(ep) = guard.as_mut() {
                            if let Some(kv) = kv_list(ep, section).get_mut(index) {
                                kv.value = v;
                            }
                        }
                    },
                }
            }
            td {
                input {
                    class: "rf-input rf-kv-input",
                    value: "{desc}",
                    oninput: move |e| {
                        let v = e.data().value();
                        let mut d = draft;
                        let mut guard = d.write();
                        if let Some(ep) = guard.as_mut() {
                            if let Some(kv) = kv_list(ep, section).get_mut(index) {
                                kv.description = v;
                            }
                        }
                    },
                }
            }
            td { class: "row-actions",
                button {
                    class: "rf-tree-action",
                    title: "删除此行",
                    onclick: move |_| {
                        let mut d = draft;
                        let mut guard = d.write();
                        if let Some(ep) = guard.as_mut() {
                            let list = kv_list(ep, section);
                            if index < list.len() {
                                list.remove(index);
                            }
                        }
                    },
                    XIcon {}
                }
            }
        }
    }
}

/// 渲染 KeyValue 编辑表（空表时显示居中空状态 + 添加按钮）。
fn kv_table(
    draft: Signal<Option<Endpoint>>,
    section: KvSection,
    item_name: &'static str,
) -> Element {
    let len = draft
        .peek()
        .as_ref()
        .map(|ep| kv_len(ep, section))
        .unwrap_or(0);
    let add_row = move |_| {
        let mut d = draft;
        let mut guard = d.write();
        if let Some(ep) = guard.as_mut() {
            kv_list(ep, section).push(KeyValue::new("", ""));
        }
    };
    if len == 0 {
        return rsx! {
            div { class: "rf-kv-empty",
                div { class: "rf-empty-icon", PlusIcon {} }
                div { class: "rf-empty-title", "暂无{item_name}" }
                button { class: "rf-btn rf-btn-sm", onclick: add_row, PlusIcon {} "添加{item_name}" }
            }
        };
    }
    rsx! {
        table { class: "kv-table",
            thead {
                tr {
                    th { "启用" }
                    th { "Key" }
                    th { "Value" }
                    th { "描述" }
                    th { "" }
                }
            }
            tbody {
                for i in 0..len {
                    { kv_row(draft, section, i) }
                }
            }
        }
        div { class: "kv-add",
            button { class: "rf-btn rf-btn-ghost rf-btn-sm", onclick: add_row, PlusIcon {} "添加{item_name}" }
        }
    }
}

/// 渲染一个 Auth 文本字段。
fn label_field(label: &'static str, draft: Signal<Option<Endpoint>>, field: AuthField) -> Element {
    auth_field(label, draft, field, None)
}

/// 渲染一个 Auth 密码字段（输入内容打码）。
fn secret_field(label: &'static str, draft: Signal<Option<Endpoint>>, field: AuthField) -> Element {
    auth_field(label, draft, field, Some("password"))
}

/// Auth 字段通用渲染。
fn auth_field(
    label: &'static str,
    draft: Signal<Option<Endpoint>>,
    field: AuthField,
    input_type: Option<&'static str>,
) -> Element {
    let value = draft
        .peek()
        .as_ref()
        .map(|ep| auth_value(&ep.request.auth, field))
        .unwrap_or_default();
    rsx! {
        div { class: "auth-field",
            label { class: "label-hint", "{label}" }
            input {
                class: "rf-input grow",
                r#type: input_type.unwrap_or("text"),
                value: "{value}",
                oninput: move |e| {
                    let v = e.data().value();
                    let mut d = draft;
                    let mut guard = d.write();
                    if let Some(ep) = guard.as_mut() {
                        set_auth_value(&mut ep.request.auth, field, v);
                    }
                },
            }
        }
    }
}

/// 响应区拖拽分隔条：按下后跟随鼠标调整响应区高度（JS 直接改内联样式，拖动流畅）。
const RESIZER_JS: &str = r#"
(function () {
  var bar = document.getElementById('rf-resizer');
  if (!bar) { return; }
  var dragging = false, startY = 0, startH = 0;
  bar.addEventListener('pointerdown', function (e) {
    dragging = true;
    startY = e.clientY;
    var resp = bar.nextElementSibling;
    startH = resp ? resp.getBoundingClientRect().height : 240;
    try { bar.setPointerCapture(e.pointerId); } catch (err) {}
    document.body.style.userSelect = 'none';
    e.preventDefault();
  });
  bar.addEventListener('pointermove', function (e) {
    if (!dragging) { return; }
    var resp = bar.nextElementSibling;
    if (!resp) { return; }
    var h = startH + (startY - e.clientY);
    h = Math.max(90, Math.min(2200, h));
    resp.style.flex = '0 0 ' + h + 'px';
    resp.style.minHeight = h + 'px';
  });
  function endDrag() {
    dragging = false;
    document.body.style.userSelect = '';
  }
  bar.addEventListener('pointerup', endDrag);
  bar.addEventListener('pointercancel', endDrag);
})();
"#;

/// 工作区页面。
#[component]
pub fn WorkspacePage() -> Element {
    let state = use_context::<AppState>();
    let mut draft: Signal<Option<Endpoint>> = use_signal(|| None);
    let active_tab: Signal<EditorTab> = use_signal(|| EditorTab::Params);
    let sending: Signal<bool> = use_signal(|| false);
    let response: Signal<Option<ResponseView>> = use_signal(|| None);
    // 响应正文视图：Pretty（默认）/ Raw。
    let resp_pretty: Signal<bool> = use_signal(|| true);
    let abort_tx: Signal<Option<tokio::sync::oneshot::Sender<()>>> = use_signal(|| None);
    let show_history: Signal<bool> = use_signal(|| false);
    let histories: Signal<Vec<RequestHistory>> = use_signal(Vec::new);
    let selected_history: Signal<Option<RequestHistory>> = use_signal(|| None);
    // M9：测试。
    let tests_input: Signal<String> = use_signal(String::new);
    let tests_parse_error: Signal<Option<String>> = use_signal(|| None);
    let test_result: Signal<Option<TestRunView>> = use_signal(|| None);
    let tests_running: Signal<bool> = use_signal(|| false);
    // M10：响应示例列表。
    let examples: Signal<Vec<ResponseExample>> = use_signal(Vec::new);
    // M11：测试历史。
    let test_history: Signal<Vec<HistoryEntry>> = use_signal(Vec::new);
    let expanded_run: Signal<Option<Uuid>> = use_signal(|| None);
    // M13：代码生成。
    let codegen_open: Signal<bool> = use_signal(|| false);
    let codegen_lang: Signal<String> = use_signal(|| "curl".to_string());
    let codegen_code: Signal<String> = use_signal(String::new);
    // M14：压测。
    let load_concurrency: Signal<String> = use_signal(|| "10".to_string());
    let load_total: Signal<String> = use_signal(|| "100".to_string());
    let load_running: Signal<bool> = use_signal(|| false);
    let load_result: Signal<Option<LoadResult>> = use_signal(|| None);
    // M15：多标签草稿缓存（按接口 id 保留每个标签页的未保存修改）。
    let tab_drafts: Signal<HashMap<Uuid, Endpoint>> = use_signal(HashMap::new);
    let dirty: Signal<HashSet<Uuid>> = use_signal(HashSet::new);
    // Body 模式切换记忆：切到「无」/表单 时暂存，切回 JSON/文本 时恢复。
    let raw_keep: Signal<String> = use_signal(String::new);
    let fields_keep: Signal<Vec<KeyValue>> = use_signal(Vec::new);
    // M17：cURL 导入。
    let curl_open: Signal<bool> = use_signal(|| false);
    let curl_input: Signal<String> = use_signal(String::new);
    // 删除二次确认弹窗（测试历史 / 响应示例）。
    let confirm: Signal<Option<(ConfirmInfo, DeleteTarget)>> = use_signal(|| None);
    // 未落库接口保存时的命名输入框。
    let save_name: Signal<String> = use_signal(String::new);
    let mut save_name_synced: Signal<Option<Uuid>> = use_signal(|| None);

    // 命名弹窗打开 / 切换接口时预填名称（防每次渲染重置用户输入）。
    {
        let mut sn = save_name;
        let mut synced = save_name_synced;
        let st = state.clone();
        use_effect(move || {
            let Some(ep) = st.pending_save.peek().as_ref().cloned() else {
                return;
            };
            if *synced.peek() != Some(ep.id) {
                synced.set(Some(ep.id));
                sn.set(ep.name);
            }
        });
    }

    // M9：draft 变化时同步 tests 配置文本。
    {
        let mut t = tests_input;
        let d = draft;
        use_effect(move || {
            let ep = d.read().clone();
            let json = ep.as_ref().and_then(|e| e.request.tests.clone());
            let text = match json {
                Some(v) => serde_json::to_string_pretty(&v).unwrap_or_else(|_| v.to_string()),
                None => String::new(),
            };
            if t.peek().as_str() != text {
                t.set(text);
            }
        });
    }

    // M15：活动标签同步 —— active_endpoint_id 变化时切换草稿（先写回旧标签缓存）。
    {
        let mut st = state.clone();
        let mut d = draft;
        let mut tds = tab_drafts;
        let mut rk = raw_keep;
        let mut fk = fields_keep;
        use_effect(move || {
            let active = *st.active_endpoint_id.read();
            // 切换前把当前草稿写回缓存（保证未保存修改不丢失）。
            if let Some(ep) = d.peek().clone() {
                tds.write().insert(ep.id, ep);
            }
            match active {
                Some(id) => {
                    let current_id = d.peek().as_ref().map(|e| e.id);
                    if current_id != Some(id) {
                        // 标签切换：清空 Body 模式切换暂存，避免暂存数据串到其他接口。
                        rk.write().clear();
                        fk.write().clear();
                        // 未持久化的导入草稿（cURL 导入）：拾取后清空，不落库。
                        let unsaved = st
                            .unsaved_draft
                            .peek()
                            .as_ref()
                            .filter(|e| e.id == id)
                            .cloned();
                        if unsaved.is_some() {
                            st.unsaved_draft.set(None);
                        }
                        let ep = tds.peek().get(&id).cloned().or(unsaved).or_else(|| {
                            let list = st.endpoints.read().clone();
                            list.into_iter().find(|e| e.id == id)
                        });
                        match ep {
                            Some(ep) => d.set(Some(ep)),
                            None => d.set(None),
                        }
                    }
                }
                None => d.set(None),
            }
        });
    }

    // M15：草稿 → 缓存 + 脏标记（对比仓库中的已保存副本）。
    {
        let mut st = state.clone();
        let d = draft;
        let mut tds = tab_drafts;
        let mut dirty = dirty;
        use_effect(move || {
            let Some(ep) = d.read().clone() else {
                return;
            };
            tds.write().insert(ep.id, ep.clone());
            // 同步全局草稿（快捷键 / 保存按钮读取最新编辑）。
            st.active_draft.set(Some(ep.clone()));
            // 未落库草稿保持最新编辑，快捷键保存时以它为准。
            if st
                .unsaved_draft
                .peek()
                .as_ref()
                .is_some_and(|u| u.id == ep.id)
            {
                st.unsaved_draft.set(Some(ep.clone()));
            }
            let saved = st.endpoints.read().iter().find(|e| e.id == ep.id).cloned();
            match saved {
                Some(s) if eq_ignoring_updated_at(&s, &ep) => {
                    dirty.write().remove(&ep.id);
                }
                _ => {
                    dirty.write().insert(ep.id);
                }
            }
        });
    }

    // M10：draft 变化时加载该接口的响应示例。
    {
        let st = state.clone();
        let d = draft;
        let ex = examples;
        use_effect(move || {
            let id = d.peek().as_ref().map(|e| e.id);
            load_examples(st.clone(), id, ex);
        });
    }

    // M11：测试结果或当前项目变化时刷新测试历史。
    {
        let st = state.clone();
        let h = test_history;
        use_effect(move || {
            let _snap = test_result.read().clone();
            let pid = *st.current_project_id.read();
            load_test_history(st.clone(), pid, h);
        });
    }

    // 关键：不在渲染中途提前 return。提前 return 会让本页面对 draft / active_endpoint_id
    // 等信号的订阅失效，导致打开/新建接口后页面永远不会重渲染（卡死在空态）。
    // 因此始终把草稿读出，编辑器主体在函数末尾以条件分支渲染。
    let ep_opt = draft.read().clone();
    let _active_endpoint_id = *state.active_endpoint_id.read();
    // 项目数据加载中标记：读取即订阅，查询期间全页显示 Spinner 遮罩，
    // 避免加载期间空白页面 / 误操作。
    let loading_flag = *state.is_loading.read();
    let loading_overlay = loading_flag.then(|| {
        rsx! {
            div { class: "rf-loading-overlay",
                div { class: "rf-spinner" }
                "正在加载项目数据…"
            }
        }
    });

    if let Some(ep) = ep_opt {
        let method_str = ep.method.to_string();
        let method_cls = ep.method.to_string().to_lowercase();
        // 拼接 URL 预览：变量经 项目 < 环境 合并后由 base_url 拼出完整地址。
        let env_id = *state.current_environment_id.read();
        let env_options: Vec<(String, String)> = state
            .environments
            .read()
            .iter()
            .map(|e| (e.id.to_string(), e.name.clone()))
            .collect();
        let env_selected: String = env_id.map(|id| id.to_string()).unwrap_or_default();
        let st_env_dd = state.clone();
        let st_env_footer = state.clone();
        let st_method = state.clone();
        let vars = state
            .current_project_id
            .peek()
            .map(|pid| merged_vars(&state, pid))
            .unwrap_or_default();
        let (full_url, _) = render_request(&ep, &vars);
        let base_prefix = full_url
            .starts_with(vars.get("base_url").map(String::as_str).unwrap_or_default())
            .then(|| {
                let base = vars.get("base_url").cloned().unwrap_or_default();
                let rest: String = full_url[base.len()..].to_string();
                (base, rest)
            });
        let body_mode = ep.request.body.mode_name();
        let auth_type = auth_mode(&ep.request.auth);
        let raw_body = body_raw(&ep);
        let active = *active_tab.read();
        let sending_visible = *sending.read();
        let resp = response.peek().clone();
        let resp_pretty_flag = *resp_pretty.peek();
        let resp_body_text = resp
            .as_ref()
            .map(|r| {
                if resp_pretty_flag {
                    r.body.clone()
                } else {
                    r.raw_body.clone()
                }
            })
            .unwrap_or_default();

        let show_history_flag = *show_history.peek();
        let history_list = histories.read().clone();
        // M9 渲染局部。
        let tests_parse_error_str = tests_parse_error.peek().clone();
        let tests_running_flag = *tests_running.peek();
        let test_run_view = test_result.peek().clone();
        let test_run_rows: Vec<TestRowView> = test_run_view
            .as_ref()
            .map(|v| v.rows.clone())
            .unwrap_or_default();
        let test_run_summary: (usize, usize, usize) = test_run_view
            .as_ref()
            .map(|v| (v.passed, v.failed, v.skipped))
            .unwrap_or((0, 0, 0));
        // M11 渲染局部。
        let expanded = *expanded_run.peek();
        let history_nodes: Vec<Element> = {
            let entries = test_history.read().clone();
            entries
            .into_iter()
            .map(|entry| {
                let time_str = entry.run.started_at.format("%m-%d %H:%M:%S").to_string();
                let rows_display = entry.view.rows.clone();
                let failure_lines: Vec<String> = rows_display
                    .iter()
                    .map(|r| r.failures.join(" | "))
                    .collect();
                let rid = entry.run.id;
                let pid = entry.run.project_id;
                let is_expanded = expanded == Some(entry.run.id);
                rsx! {
                    div { class: "hist-row",
                        span { class: "hist-time", "{time_str}" }
                        span { class: "hist-name", "{entry.view.name}" }
                        span { class: "test-summary ok", "通过 {entry.view.passed}" }
                        span { class: "test-summary bad", "失败 {entry.view.failed}" }
                        if entry.view.skipped > 0 {
                            span { class: "test-summary skip", "跳过 {entry.view.skipped}" }
                        }
                        div { class: "spacer" }
                        button {
                            class: "rf-btn rf-btn-sm",
                            onclick: move |_| {
                                let mut ex = expanded_run;
                                let cur = *ex.peek();
                                ex.set(if cur == Some(rid) { None } else { Some(rid) });
                            },
                            if is_expanded { "收起" } else { "详情" }
                        }
                        button {
                            class: "rf-btn rf-btn-sm",
                            onclick: move |_| {
                                let mut cd = confirm;
                                cd.set(Some((
                                    ConfirmInfo::new(
                                        "删除测试历史",
                                        "确定要删除这条测试历史吗？此操作不可恢复。",
                                    ),
                                    DeleteTarget::TestHistory { run_id: rid, project_id: pid },
                                )));
                            },
                            "删除"
                        }
                        if is_expanded {
                            div { class: "hist-detail",
                                for (i2, row) in rows_display.into_iter().enumerate() {
                                    div { class: "kv-row", key: "{i2}",
                                        span { class: "rf-method rf-method-chip rf-method-chip-{row.method.to_lowercase()}", "{row.method}" }
                                        span { class: "url", "{row.path}" }
                                        span { class: "test", "{row.name}" }
                                        div { class: "spacer" }
                                        if let Some(e) = &row.error {
                                            span { class: "warn-text", "{e}" }
                                        } else if !row.ok && !row.skipped {
                                            span { class: "test-badge bad", "失败" }
                                        }
                                        if !failure_lines[i2].is_empty() {
                                            span { class: "warn-text", "{failure_lines[i2]}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            })
            .collect()
        };
        // M10 渲染局部。
        let st = state.clone();
        let st_export = st.clone();
        let st_save = st.clone();
        let resp_view = resp.clone();
        // M13 渲染局部。
        let st_cg = state.clone();
        let st_cg_copy = st_cg.clone();
        // M18 渲染局部：未落库接口保存命名弹窗。
        // 注意必须用 read()（订阅信号）而非 peek()：弹窗开关与输入框值依赖重渲染。
        let pending_save_ep = (*state.pending_save.read()).clone();
        let pending_save = state.pending_save;
        let save_name_str = save_name.read().clone();
        let st_confirm_save = state.clone();
        let st_btn = state.clone();
        let codegen_open_flag = *codegen_open.read();
        // M17 渲染局部。
        let curl_open_flag = *curl_open.read();
        let codegen_lang_str = codegen_lang.peek().clone();
        let codegen_code_str = codegen_code.peek().clone();
        // M14 渲染局部。
        let st_load = state.clone();
        let load_concurrency_str = load_concurrency.peek().clone();
        let load_total_str = load_total.peek().clone();
        let load_running_flag = *load_running.peek();
        let load_result_flag = load_result.peek().clone();
        // M15 渲染局部。
        let tab_ids: Vec<Uuid> = state.open_tabs.read().clone();
        let active_id_flag: Option<Uuid> = *state.active_endpoint_id.peek();
        let dirty_flag: HashSet<Uuid> = dirty.peek().clone();
        let tab_infos: Vec<(Uuid, String, bool)> = tab_ids
            .iter()
            .map(|id| {
                let title = tab_drafts
                    .peek()
                    .get(id)
                    .map(|e| {
                        if e.name.trim().is_empty() {
                            format!("{} {}", e.method, e.path)
                        } else {
                            e.name.clone()
                        }
                    })
                    .unwrap_or_else(|| "接口".to_string());
                (*id, title, dirty_flag.contains(id))
            })
            .collect();
        let tab_nodes: Vec<Element> = tab_infos
            .iter()
            .map(|(tid, title, is_dirty)| {
                let st_row = state.clone();
                let st_close = state.clone();
                let tid = *tid;
                let title = title.clone();
                let is_dirty = *is_dirty;
                let is_active = active_id_flag == Some(tid);
                rsx! {
                    div {
                        class: if is_active { "editor-tab active" } else { "editor-tab" },
                        key: "{tid}",
                        onclick: move |_| st_row.open_endpoint_tab(tid),
                        span { "{title}" }
                        if is_dirty {
                            span { class: "tab-dirty", "●" }
                        }
                        button {
                            class: "rf-tab-close",
                            onclick: move |e| {
                                e.stop_propagation();
                                close_tab_impl(
                                    st_close.clone(),
                                    tid,
                                    st_close.open_tabs,
                                    tab_drafts,
                                    dirty,
                                    st_close.active_endpoint_id,
                                    draft,
                                );
                            },
                            "×"
                        }
                    }
                }
            })
            .collect();
        let examples_list = examples.read().clone();
        let example_nodes: Vec<Element> = examples_list.iter().map(|ex_item| {
        let eid = ex_item.id;
        let ex_name = ex_item.name.clone();
        rsx! {
            div { class: "ex-row", key: "{eid}",
                span { class: if ex_item.status < 400 { "status-ok" } else { "status-err" }, "{ex_item.status}" }
                span { "{ex_item.name}" }
                span { class: "hint", "{ex_item.content_type}" }
                div { class: "spacer" }
                button {
                    class: "rf-btn rf-btn-sm",
                    onclick: move |_| {
                        let mut cd = confirm;
                        cd.set(Some((
                            ConfirmInfo::new(
                                "删除响应示例",
                                format!("确定要删除响应示例「{ex_name}」吗？此操作不可恢复。"),
                            ),
                            DeleteTarget::ResponseExample { example_id: eid, endpoint_id: ep.id },
                        )));
                    },
                    "删除"
                }
            }
        }
    }).collect();
        let enabled_params: Vec<KeyValue> = ep
            .request
            .params
            .iter()
            .filter(|k| k.enabled)
            .cloned()
            .collect();
        let enabled_headers: Vec<KeyValue> = ep
            .request
            .headers
            .iter()
            .filter(|k| k.enabled)
            .cloned()
            .collect();

        let mut set_tests_input = {
            let mut d = draft;
            let mut t = tests_input;
            let mut err = tests_parse_error;
            move |v: String| {
                t.set(v.clone());
                if v.trim().is_empty() {
                    err.set(None);
                    let mut guard = d.write();
                    if let Some(ep) = guard.as_mut() {
                        ep.request.tests = None;
                    }
                    return;
                }
                match serde_json::from_str::<serde_json::Value>(&v) {
                    Ok(val) => {
                        err.set(None);
                        let mut guard = d.write();
                        if let Some(ep) = guard.as_mut() {
                            ep.request.tests = Some(val);
                        }
                    }
                    Err(e) => err.set(Some(format!("测试配置 JSON 无效：{e}"))),
                }
            }
        };

        // M9：运行单接口测试（用当前草稿）。
        let run_current_tests = {
            let st = state.clone();
            let d = draft;
            let tr = test_result;
            let run = tests_running;
            move || {
                let Some(ep) = d.peek().clone() else {
                    st.toast_error("没有可测试的接口");
                    return;
                };
                run_tests(st.clone(), vec![ep], "接口测试".into(), tr, run);
            }
        };
        // M9：运行当前文件夹测试。
        let run_folder_tests = {
            let st = state.clone();
            let d = draft;
            let tr = test_result;
            let run = tests_running;
            move || {
                let Some(ep) = d.peek().clone() else {
                    st.toast_error("没有可测试的接口");
                    return;
                };
                let Some(fid) = ep.folder_id else {
                    st.toast_error("当前接口不在文件夹中");
                    return;
                };
                let list: Vec<Endpoint> = st
                    .endpoints
                    .read()
                    .iter()
                    .filter(|e| e.folder_id == Some(fid))
                    .cloned()
                    .collect();
                run_tests(st.clone(), list, "文件夹测试".into(), tr, run);
            }
        };
        // M9：运行整个项目测试。
        let run_project_tests = {
            let st = state.clone();
            let tr = test_result;
            let run = tests_running;
            move || {
                let Some(project_id) = *st.current_project_id.peek() else {
                    st.toast_error("未选择项目");
                    return;
                };
                let list: Vec<Endpoint> = st
                    .endpoints
                    .read()
                    .iter()
                    .filter(|e| e.project_id == project_id)
                    .cloned()
                    .collect();
                run_tests(st.clone(), list, "项目测试".into(), tr, run);
            }
        };

        let mut save = {
            let st = state.clone();
            let d = draft;
            let mut dirty = dirty;
            move || {
                let Some(ep) = d.peek().clone() else {
                    st.toast_error("没有可保存的接口");
                    return;
                };
                if ep.name.trim().is_empty() {
                    st.toast_error("接口名称不能为空");
                    return;
                }
                let id = ep.id;
                let name = ep.name.clone();
                tracing::info!("用户保存接口 id={id} name={name}");
                st.record_step(format!("保存接口「{name}」"));
                // 未落库接口统一走命名确认弹框；已落库直接保存当前草稿。
                st.save_active_endpoint();
                dirty.write().remove(&id);
            }
        };

        // M15：新建标签。
        let mut new_tab = {
            let st = state.clone();
            let mut tds = tab_drafts;
            let mut d = draft;
            let mut ts = st.open_tabs;
            let mut active = st.active_endpoint_id;
            move || {
                let Some(project_id) = *st.current_project_id.peek() else {
                    st.toast_error("未选择项目");
                    return;
                };
                let ep = blank_endpoint(project_id);
                tds.write().insert(ep.id, ep.clone());
                ts.write().push(ep.id);
                active.set(Some(ep.id));
                d.set(Some(ep));
            }
        };

        // M15：关闭标签由自由函数 close_tab_impl 处理（标签栏循环内调用）。

        let mut fmt_json = {
            let st = state.clone();
            let mut d = draft;
            move || {
                let raw = d.peek().as_ref().and_then(|ep| match &ep.request.body {
                    BodySpec::Json { raw } => Some(raw.clone()),
                    _ => None,
                });
                match raw {
                    Some(raw) => match format_json(&raw) {
                        Ok(pretty) => {
                            let mut guard = d.write();
                            if let Some(ep) = guard.as_mut() {
                                if let BodySpec::Json { raw } = &mut ep.request.body {
                                    *raw = pretty;
                                }
                            }
                        }
                        Err(e) => st.toast_error(format!("JSON 格式错误：{}", e.user_message())),
                    },
                    None => st.toast_error("当前不是 JSON Body"),
                }
            }
        };

        let mut set_body_raw = {
            let mut d = draft;
            move |v: String| {
                let mut guard = d.write();
                if let Some(ep) = guard.as_mut() {
                    match &mut ep.request.body {
                        BodySpec::Json { raw } | BodySpec::Text { raw } => *raw = v,
                        _ => {}
                    }
                }
            }
        };

        // M5：发送请求。
        let mut do_send = {
            let st = state.clone();
            let d = draft;
            let mut sending = sending;
            let mut abort_tx = abort_tx;
            move || {
                if *sending.peek() {
                    return;
                }
                sending.set(true);
                let Some(ep) = d.peek().clone() else {
                    sending.set(false);
                    st.toast_error("没有可发送的接口");
                    return;
                };
                let Some(project_id) = *st.current_project_id.peek() else {
                    sending.set(false);
                    st.toast_error("未选择项目");
                    return;
                };
                let vars = merged_vars(&st, project_id);
                let (url, spec) = render_request(&ep, &vars);
                if !is_absolute_url(&url) {
                    sending.set(false);
                    tracing::warn!("[HTTP] URL 不是完整地址，且未配置 base_url: {url}");
                    st.toast_error("请输入完整的 URL，或在环境变量中配置 base_url");
                    return;
                }
                let method = ep.method;
                tracing::info!("[HTTP] 准备发送请求: {} {}", method, url);
                st.record_step(format!("发送请求 {} {}", method, url));
                let (tx, rx) = tokio::sync::oneshot::channel();
                abort_tx.set(Some(tx));
                sending.set(true);
                let db = st.services.db.clone();
                let st_task = st.clone();
                let mut sg = sending;
                let mut rv = response;
                let mut ht = histories;
                spawn(async move {
                    // 先 yield 确保 UI 已渲染发送中状态（转圈 + 取消按钮）。
                    tokio::task::yield_now().await;
                    // 无论成功 / 失败 / 取消，最后统一恢复按钮状态。
                    let outcome = tokio::select! {
                        _ = rx => None,
                        result = send_request(method, &url, &spec, None) => Some(result),
                    };
                    sg.set(false);
                    match outcome {
                        None => {
                            st_task.toast_info("请求已取消");
                        }
                        Some(Ok(data)) => {
                            tracing::info!(
                                "[HTTP] 请求完成: status={}, duration={}ms",
                                data.status,
                                data.duration_ms
                            );
                            // OAuth2：请求过程可能自动刷新了 token（缓存更新），
                            // 若草稿仍是旧令牌则静默持久化最新令牌。
                            if let AuthSpec::OAuth2 {
                                client_id,
                                token_url,
                                token: spec_token,
                                ..
                            } = &spec.auth
                            {
                                if let Some(cached) = fox_oauth::cached_token(client_id, token_url)
                                {
                                    if spec_token.as_ref() != Some(&cached) {
                                        let mut refreshed = ep.clone();
                                        if let AuthSpec::OAuth2 { token, .. } =
                                            &mut refreshed.request.auth
                                        {
                                            *token = Some(cached.clone());
                                        }
                                        st_task.save_endpoint_quiet(refreshed);
                                    }
                                }
                            }
                            let history = build_history(&ep, project_id, &url, &data);
                            rv.set(to_response_view(data));
                            let db = db.clone();
                            if let Err(e) = repo::save_request_history(&db, &history).await {
                                st_task.toast_error(format!("保存历史失败：{}", e.user_message()));
                            } else {
                                st_task.toast_success("请求完成，已保存历史");
                            }
                            if let Ok(list) =
                                repo::list_request_histories(&db, project_id, 50).await
                            {
                                ht.set(list);
                            }
                        }
                        Some(Err(e)) => {
                            tracing::error!("[HTTP] 请求失败: {}", e);
                            let msg = match &e {
                                AppError::Http(re) => describe_http_error(re),
                                _ => e.user_message(),
                            };
                            st_task.toast_error(format!("请求失败：{msg}"));
                        }
                    }
                });
            }
        };

        // M5：取消请求。
        let mut cancel_send = {
            let mut abort_tx = abort_tx;
            let mut sending = sending;
            move || {
                if let Some(tx) = abort_tx.write().take() {
                    let _ = tx.send(());
                }
                sending.set(false);
            }
        };

        // M17：导入 cURL 命令 —— 解析后覆盖当前草稿。
        let mut import_curl = {
            let st = state.clone();
            let mut d = draft;
            let mut tds = tab_drafts;
            let mut dirty = dirty;
            let mut co = curl_open;
            let mut ci = curl_input;
            move || {
                let Some(_ep) = d.peek().clone() else {
                    st.toast_error("未选择接口，无法导入");
                    return;
                };
                let raw = ci.peek().clone();
                let parsed = match parse_curl(&raw) {
                    Ok(p) => p,
                    Err(e) => {
                        st.toast_error(format!("cURL 格式无法识别：{}", e.user_message()));
                        return;
                    }
                };
                let mut guard = d.write();
                if let Some(ep) = guard.as_mut() {
                    apply_curl(ep, &parsed);
                }
                drop(guard);
                let ep2 = d.read().clone();
                if let Some(ep2) = ep2 {
                    tds.write().insert(ep2.id, ep2.clone());
                    dirty.write().insert(ep2.id);
                    st.save_endpoint(ep2);
                    co.set(false);
                    ci.set(String::new());
                    return;
                }
            }
        };

        rsx! {
                    div { class: "editor",
                        // M15：多标签栏。
                        div { class: "tab-bar",
                            for node in tab_nodes {
                                { node }
                            }
                            button {
                                id: "tab-add-endpoint",
                                class: "rf-btn tab-add-btn",
                                title: "新建接口",
                                onclick: move |_| new_tab(),
                                PlusIcon {}
                            }
                            div { class: "tab-bar-right",
                                Dropdown {
                                    class: "tab-env-dd",
                                    options: env_options.clone(),
                                    selected: env_selected.clone(),
                                    placeholder: "未选环境",
                                    on_select: move |v: String| {
                                        let id = uuid::Uuid::parse_str(&v).ok();
                                        st_env_dd.select_environment(id);
                                    },
                                    footer: Some(rsx! {
                                        div {
                                            class: "rf-dropdown-footer",
                                            button {
                                                class: "rf-dropdown-action",
                                                onclick: move |_| {
                                                    let mut pg = st_env_footer.current_page;
                                                    pg.set(Page::Settings);
                                                },
                                                "管理环境"
                                            }
                                        }
                                    }),
                                }
                            }
                        }
                        div { class: "url-bar",
                            div { class: "ub-group",
                                Dropdown {
                                    class: "rf-dd-method rf-dd-method-{method_cls}",
                                    options: HttpMethod::all()
                                        .iter()
                                        .map(|m| (m.to_string(), m.to_string()))
                                        .collect(),
                                    selected: method_str.clone(),
                                    on_select: move |v: String| {
                                        if let Ok(m) = v.parse::<HttpMethod>() {
                                            let st = st_method.clone();
                                            let mut d = draft;
                                            let mut guard = d.write();
                                            if let Some(ep) = guard.as_mut() {
                                                ep.method = m;
                                                let ep_save = ep.clone();
                                                drop(guard);
                                                let id = ep_save.id;
                                                st.save_endpoint(ep_save);
                                                tracing::info!("自动保存方法 id={} method={}", id, m);
                                            }
                                        }
                                    },
                                }
                                if let Some((base, _)) = &base_prefix {
                                    span { class: "ub-base-url", title: "base_url（请在设置-环境管理中修改）", "{base}" }
                                }
                                input {
                                    class: "rf-input ub-url-input",
                                    value: "{ep.path}",
                                    placeholder: "请求路径，如 /api/users 或 {{base_url}}/api/users",
                                    oninput: move |e| {
                                        let v = e.data().value();
                                        let mut d = draft;
                                        let mut guard = d.write();
                                        if let Some(ep) = guard.as_mut() { ep.path = v; }
                                    },
                                }
                                button {
                                    class: "ub-send",
                                    disabled: sending_visible,
                                    title: "发送请求（Ctrl+Enter）",
                                    onclick: move |_| do_send(),
                                    if sending_visible {
                                        span { class: "rf-spin-sm" }
                                        "发送中"
                                    } else {
                                        SendIcon {}
                                        "发送"
                                    }
                                }
                            }
                            if sending_visible {
                                button {
                                    class: "rf-btn rf-btn-ghost rf-btn-sm",
                                    onclick: move |_| cancel_send(),
                                    "取消",
                                }
                            }
div { class: "ub-actions",
                                 button {
                                     class: "rf-btn rf-btn-ghost ub-btn",
                                     title: "保存接口",
                                    onclick: move |_| save(),
                                    SaveIcon {}
                                }
                                button {
                                    class: "rf-btn rf-btn-ghost ub-btn",
                                    title: "生成客户端代码",
                                    onclick: move |_| {
                                        let Some(ep) = draft.peek().clone() else {
                                            return;
                                        };
                                        let lang_str = codegen_lang.peek().clone();
                                        let lang = Lang::from_str_cn(&lang_str).unwrap_or(Lang::Curl);
                                        if let Some(code) = build_codegen_code(&st_btn, &ep, lang) {
                                            let mut cc = codegen_code;
                                            cc.set(code);
                                        }
                                        let mut co = codegen_open;
                                        co.set(true);
                                    },
                                    CodeIcon {}
                                }
                            }
                        }
                        div { class: "tabs",
                            for (tab_enum, label, count) in [
                                (EditorTab::Params, "Params", ep.request.params.len()),
                                (EditorTab::Headers, "Headers", ep.request.headers.len()),
                                (EditorTab::Body, "Body", usize::from(body_mode != "none")),
                                (EditorTab::Auth, "Auth", usize::from(auth_type != "none")),
                                (EditorTab::Tests, "Tests", usize::from(ep.request.tests.is_some())),
                                (EditorTab::Docs, "Docs", 0),
                            ] {
                                button {
                                    class: if active == tab_enum { "rf-tab active" } else { "rf-tab" },
                                    onclick: move |_| {
                                        let mut t = active_tab;
                                        t.set(tab_enum);
                                    },
                                    "{label}"
                                    if count > 0 {
                                        span { class: "rf-tab-badge", "{count}" }
                                    }
                                }
                            }
                            div { class: "spacer" }
                            button {
                                class: "rf-tab rf-tab-icon",
                                title: "请求历史（抽屉）",
                                onclick: move |_| {
                                    load_history_list(state.clone(), histories);
                                    let mut sh = show_history;
                                    sh.set(true);
                                },
                                ClockIcon {}
                            }
                        }
                        div { class: "tab-body",
                            match active {
                                EditorTab::Params => rsx! {
                                    { kv_table(draft, KvSection::Params, "参数") }
                                },
                                EditorTab::Headers => rsx! {
                                    { kv_table(draft, KvSection::Headers, "请求头") }
                                },
                                EditorTab::Body => rsx! {
                                    div { class: "row rf-mb-2",
                                        Dropdown {
                                            options: vec![
                                                ("none".into(), "无".into()),
                                                ("json".into(), "JSON".into()),
                                                ("text".into(), "文本".into()),
                                                ("urlencoded".into(), "表单 (x-www-form-urlencoded)".into()),
                                            ],
                                            selected: body_mode,
                                            on_select: move |v: String| {
                                                let mut d = draft;
                                                let mut rk = raw_keep;
                                                let mut fk = fields_keep;
                                                let mut guard = d.write();
                                                if let Some(ep) = guard.as_mut() {
                                                    let mut rk_guard = rk.write();
                                                    let mut fk_guard = fk.write();
                                                    let next = switch_body_mode_keep(
                                                        &ep.request.body,
                                                        &v,
                                                        &mut rk_guard,
                                                        &mut fk_guard,
                                                    );
                                                    ep.request.body = next;
                                                }
                                            },
                                        }
                                        if body_mode == "json" {
                                            button { class: "rf-btn rf-btn-sm", onclick: move |_| fmt_json(), "格式化 JSON" }
                                        }
                                    }
                                    if body_mode == "none" {
                                        div { class: "empty", "暂无请求体" }
                                    } else if body_mode == "urlencoded" {
                                        { kv_table(draft, KvSection::UrlEncoded, "字段") }
                                    } else {
                                        div { class: "body-editor-fl",
                                            pre { id: "body-pre-hl", class: "body-editor-hl", {highlight_json(raw_body.clone())} }
                                            textarea {
                                                id: "body-textarea-input",
                                                class: "rf-textarea rf-body-editor",
                                                placeholder: "文本请求体",
                                                value: "{raw_body}",
                                                oninput: move |e| set_body_raw(e.data().value()),
                                                onscroll: move |_e| {
                                                    eval(
                                                        "(function(){var ta=document.getElementById('body-textarea-input');var hl=document.getElementById('body-pre-hl');if(ta&&hl){hl.scrollTop=ta.scrollTop;hl.scrollLeft=ta.scrollLeft;}})();",
                                                    );
                                                },
                                            }
                                        }
                                    }
                                },
                                EditorTab::Auth => {
                                    let st_oauth_authorize = state.clone();
                                    let st_oauth_clear = state.clone();
                                    rsx! {
                                    div { class: "row rf-mb-2",
                                        Dropdown {
                                            options: vec![
                                                ("none".into(), "无认证".into()),
                                                ("bearer".into(), "Bearer Token".into()),
                                                ("basic".into(), "Basic Auth".into()),
                                                ("apikey".into(), "API Key".into()),
                                                ("oauth2".into(), "OAuth 2.0".into()),
                                            ],
                                            selected: auth_type,
                                            on_select: move |v: String| {
                                                let mut d = draft;
                                                let mut guard = d.write();
                                                if let Some(ep) = guard.as_mut() {
                                                    let cur = switch_auth(&ep.request.auth, &v);
                                                    ep.request.auth = cur;
                                                }
                                            },
                                        }
                                    }
                                    match &ep.request.auth {
                                        AuthSpec::None => rsx! { div { class: "empty", "无认证" } },
                                        AuthSpec::Bearer { .. } => rsx! {
                                            { label_field("Token", draft, AuthField::BearerToken) }
                                        },
                                        AuthSpec::Basic { .. } => rsx! {
                                            { label_field("用户名", draft, AuthField::BasicUser) }
                                            { label_field("密码", draft, AuthField::BasicPass) }
                                        },
                                        AuthSpec::ApiKey { location, .. } => rsx! {
                                            { label_field("Key 名称", draft, AuthField::ApiKeyName) }
                                            { label_field("Key 值", draft, AuthField::ApiKeyValue) }
                                            div { class: "row rf-mt-2",
                                                span { class: "label-hint", "放置位置：" }
                                                Dropdown {
                                                    options: vec![
                                                        ("header".into(), "请求头".into()),
                                                        ("query".into(), "查询参数".into()),
                                                    ],
                                                    selected: loc_name(*location).to_string(),
                                                    on_select: move |v: String| {
                                                        let mut d = draft;
                                                        let mut guard = d.write();
                                                        if let Some(ep) = guard.as_mut() {
                                                            if let AuthSpec::ApiKey { location, .. } = &mut ep.request.auth {
                                                                *location = if v == "query" { ApiKeyLocation::Query } else { ApiKeyLocation::Header };
                                                            }
                                                        }
                                                    },
                                                }
                                            }
                                        },
                                        AuthSpec::OAuth2 { .. } => rsx! {
                                            div { class: "oauth-status", { oauth_status_ui(&ep) } }
                                            { label_field("Client ID", draft, AuthField::OAuth2ClientId) }
                                            { secret_field("Client Secret", draft, AuthField::OAuth2ClientSecret) }
                                            { label_field("授权地址 Auth URL", draft, AuthField::OAuth2AuthUrl) }
                                            { label_field("令牌地址 Token URL", draft, AuthField::OAuth2TokenUrl) }
                                            { label_field("Scope（空格分隔）", draft, AuthField::OAuth2Scope) }
                                            { label_field("回调地址 Redirect URI", draft, AuthField::OAuth2RedirectUri) }
                                            div { class: "row rf-mt-2",
                                                button {
                                                    class: "rf-btn rf-btn-primary rf-btn-sm",
                                                    onclick: move |_| authorize_oauth2(st_oauth_authorize.clone(), draft),
                                                    "立即授权"
                                                }
                                                button {
                                                    class: "rf-btn rf-btn-sm rf-btn-ghost",
                                                    onclick: move |_| clear_oauth2_token(st_oauth_clear.clone(), draft),
                                                    "清除令牌"
                                                }
                                            }
                                            div { class: "oauth-hint",
                                                "授权码流流程：点击「立即授权」→ 浏览器打开授权页 → 授权后自动回调本机 9090 端口换取 token；token 过期后请求时自动用 refresh_token 静默刷新。"
                                            }
                                        },
                                    }
                                    }
                                },
                                EditorTab::Docs => rsx! {
                                    div { class: "docs-meta",
                                        span { class: "method-badge", "{ep.method}" }
                                        span { class: "doc-path", "{ep.path}" }
                                        div { class: "spacer" }
                                        button {
                                            class: "rf-btn rf-btn-sm rf-btn-primary",
                                            onclick: move |_| export_markdown_file(st_export.clone()),
                                            "导出项目 Markdown"
                                        }
                                    }
                                    if ep.description.is_empty() {
                                        div { class: "empty", "暂无描述" }
                                    } else {
                                        div { class: "doc-desc", "{ep.description}" }
                                    }
                                    div { class: "docs-block",
                                        h4 { "查询参数" }
                                        if enabled_params.is_empty() {
                                            div { class: "hint", "无启用的查询参数" }
                                        } else {
                                            for kv in enabled_params {
                                                div { class: "kv-row",
                                                    code { "{kv.key}" }
                                                    span { "{kv.value}" }
                                                }
                                            }
                                        }
                                        h4 { "请求头" }
                                        if enabled_headers.is_empty() {
                                            div { class: "hint", "无启用的请求头" }
                                        } else {
                                            for kv in enabled_headers {
                                                div { class: "kv-row",
                                                    code { "{kv.key}" }
                                                    span { "{kv.value}" }
                                                }
                                            }
                                        }
                                        h4 { "请求体" }
                                        match &ep.request.body {
                                            BodySpec::Json { raw, .. } => rsx! { pre { class: "doc-body", {highlight_json(raw.clone())} } },
                                            BodySpec::UrlEncoded { .. } => rsx! { div { class: "hint", "URL 编码表单参数" } },
                                            BodySpec::Text { raw, .. } => rsx! { pre { class: "doc-body", "{raw}" } },
                                            BodySpec::Multipart { .. } => rsx! { div { class: "hint", "Multipart 表单" } },
                                            BodySpec::GraphQL { .. } => rsx! { div { class: "hint", "GraphQL 查询（POST JSON）" } },
                                            BodySpec::None => rsx! { div { class: "hint", "无请求体" } },
                                        }
                                        h4 { "认证" }
                                        div { class: "kv-row", span { class: "hint", "{auth_type}" } }
                                        h4 { "响应示例" }
                                        if example_nodes.is_empty() {
                                            div { class: "hint", "暂无响应示例（发送请求后点击「保存为示例」）" }
                                        } else {
                                            for node in example_nodes {
                                                { node }
                                            }
                                        }
                                    }
            },
                                EditorTab::Tests => rsx! {
                                    div { class: "row rf-mb-2",
                                        button {
                                            class: "rf-btn rf-btn-sm rf-btn-primary",
                                            disabled: tests_running,
                                            onclick: move |_| run_current_tests(),
                                            "运行测试"
                                        }
                                        button {
                                            class: "rf-btn rf-btn-sm",
                                            disabled: tests_running,
                                            onclick: move |_| run_folder_tests(),
                                            "运行文件夹测试"
                                        }
                                        button {
                                            class: "rf-btn rf-btn-sm",
                                            disabled: tests_running,
                                            onclick: move |_| run_project_tests(),
                                            "运行项目测试"
                                        }
                                        div { class: "spacer" }
                                        span { class: "hint-inline", "配置写入 request_json.tests 后保存接口生效" }
                                    }
                                    textarea {
                                        class: "rf-textarea rf-oapi-input",
                                        placeholder: "测试配置 JSON：pre_request / extract / assertions（保存接口后生效）",
                                        value: "{tests_input}",
                                        oninput: move |e| set_tests_input(e.data().value()),
                                    }
                                    if let Some(e) = &tests_parse_error_str {
                                        div { class: "warn-text", "{e}" }
                                    }
                                    if !tests_running_flag {
                                        div { class: "hint", "变量提取结果会在本次运行中按顺序传递给后续接口" }
                                    }
                                    div { class: "history-box",
                                        div { class: "kv-title", "历史测试（最近 20 次）" }
                                        if history_nodes.is_empty() {
                                            div { class: "hint", "暂无测试历史，运行测试后自动记录" }
                                        } else {
                                            for node in history_nodes {
                                                { node }
                                            }
                                        }
                                    }
                                    // M14：压测区。
                                    div { class: "load-box",
                                        div { class: "row rf-mb-2",
                                            span { class: "hint-inline", "压测（并发基准）" }
                                            input {
                                                class: "rf-input rf-input-sm rf-in-72",
                                                value: "{load_concurrency_str}",
                                                oninput: move |e| {
                                                    let v = e.data().value();
                                                    let mut lc = load_concurrency;
                                                    lc.set(v);
                                                },
                                            }
                                            span { class: "hint-inline", "并发" }
                                            input {
                                                class: "rf-input rf-input-sm rf-in-96",
                                                value: "{load_total_str}",
                                                oninput: move |e| {
                                                    let v = e.data().value();
                                                    let mut lt = load_total;
                                                    lt.set(v);
                                                },
                                            }
                                            span { class: "hint-inline", "总次数" }
                                            button {
                                                class: "rf-btn rf-btn-sm rf-btn-primary",
                                                disabled: load_running_flag,
                                                onclick: move |_| {
        let Some(ep) = draft.read().clone() else {
                                                        return;
                                                    };
                                                    let concurrency: usize = load_concurrency
                                                        .peek()
                                                        .trim()
                                                        .parse()
                                                        .unwrap_or(10)
                                                        .max(1);
                                                    let total: usize = load_total
                                                        .peek()
                                                        .trim()
                                                        .parse()
                                                        .unwrap_or(100)
                                                        .max(1);
                                                    run_load_benchmark(
                                                        st_load.clone(),
                                                        ep,
                                                        concurrency,
                                                        total,
                                                        load_result,
                                                        load_running,
                                                        test_result,
                                                    );
                                                },
                                                if load_running_flag { "压测中……" } else { "开始压测" }
                                            }
                                        }
                                        if let Some(lr) = &load_result_flag {
                                            div { class: "load-result",
                                                div { class: "kv-title", "压测结果" }
                                                div { class: "load-grid",
                                                    span { "请求总数" }
                                                    span { "{lr.total}" }
                                                    span { "成功 / 失败" }
                                                    span { class: if lr.failed == 0 { "load-ok" } else { "load-bad" },
                                                        "{lr.ok} / {lr.failed}"
                                                    }
                                                    span { "总耗时" }
                                                    span { "{lr.total_ms} ms" }
                                                    span { "QPS" }
                                                    span { "{lr.rps:.1}" }
                                                    span { "平均耗时" }
                                                    span { "{lr.avg_ms:.1} ms" }
                                                    span { "P50 / P90 / P99" }
                                                    span { "{lr.p50_ms} / {lr.p90_ms} / {lr.p99_ms} ms" }
                                                }
                                                if !lr.errors.is_empty() {
                                                    div { class: "warn-text", "错误示例：" }
                                                    for e in &lr.errors {
                                                        div { class: "load-err", "{e}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                },
                            }
                        }
                        // M9：测试结果区（失败高亮）。
                        if test_run_view.is_some() {
                            div { class: "test-run",
                                div { class: "resp-head",
                                    span { class: "label-hint", "测试结果" }
                                    span { class: "test-summary ok", "通过 {test_run_summary.0}" }
                                    span { class: "test-summary bad", "失败 {test_run_summary.1}" }
                                    if test_run_summary.2 > 0 {
                                        span { class: "test-summary skip", "跳过 {test_run_summary.2}" }
                                    }
                                }
                                for (idx, row) in test_run_rows.clone().into_iter().enumerate() {
                                    div { key: "{idx}", class: if !row.ok && !row.skipped { "test-row fail" } else if row.skipped { "test-row skip" } else { "test-row" },
                                        div { class: "test-row-main",
                                            span { class: "rf-method rf-method-chip rf-method-chip-{row.method.to_lowercase()}", "{row.method}" }
                                            span { class: "url", "{row.path}" }
                                            span { class: "test", "{row.name}" }
                                            div { class: "spacer" }
                                            if !row.ok && !row.skipped {
                                                span { class: "test-badge bad", "失败" }
                                            } else if row.skipped {
                                                span { class: "test-badge skip", "跳过" }
                                            } else {
                                                span { class: "test-badge ok", "通过" }
                                            }
                                            span { "{row.duration_ms.unwrap_or(0)} ms" }
                                        }
                                        if let Some(err) = row.error {
                                            div { class: "test-fail", "{err}" }
                                        }
                                        for f in row.failures {
                                            div { class: "test-fail", XIcon {} "{f}" }
                                        }
                                    }
                                }
                            }
                        }
                        // M5：响应区。
                        if sending_visible || resp.is_some() {
                            div {
                                id: "rf-resizer",
                                class: "rf-resizer",
                                title: "拖动调整响应区高度",
                                onmounted: move |_| {
                                    let _ = eval(RESIZER_JS);
                                },
                            }
                            div { class: "response",
                                div { class: "resp-head",
                                    if let Some(r) = &resp {
                                        {
                                            let status_cls = if r.status < 300 {
                                                "rf-status-2xx"
                                            } else if r.status < 400 {
                                                "rf-status-3xx"
                                            } else if r.status < 500 {
                                                "rf-status-4xx"
                                            } else {
                                                "rf-status-5xx"
                                            };
                                            let size_str = human_size(r.size_bytes);
                                            let body_text = resp_body_text.clone();
                                            let st_cp = st_save.clone();
                                            let st_ex = st_save.clone();
                                            rsx! {
                                                span { class: "rf-status-pill {status_cls}", "{r.status}" }
                                                div { class: "resp-stats",
                                                    span { title: "耗时", "{r.duration_ms} ms" }
                                                    span { title: "响应大小", "{size_str}" }
                                                    if r.truncated {
                                                        span { class: "warn-pill", "已截断（>20MB）" }
                                                    }
                                                }
                                                span { class: "resp-ctype", "{r.content_type}" }
                                                div { class: "spacer" }
                                                div { class: "rf-seg",
                                                    button {
                                                        class: if resp_pretty_flag { "active" } else { "" },
                                                        onclick: move |_| {
                                                            let mut p = resp_pretty;
                                                            p.set(true);
                                                        },
                                                        "Pretty"
                                                    }
                                                    button {
                                                        class: if resp_pretty_flag { "" } else { "active" },
                                                        onclick: move |_| {
                                                            let mut p = resp_pretty;
                                                            p.set(false);
                                                        },
                                                        "Raw"
                                                    }
                                                }
                                                button {
                                                    class: "rf-btn rf-btn-ghost rf-btn-sm",
                                                    title: "复制响应正文",
                                                    onclick: move |_| {
                                                        copy_text(&body_text);
                                                        st_cp.toast_success("响应正文已复制");
                                                    },
                                                    CopyIcon {}
                                                    "复制"
                                                }
                                                button {
                                                    class: "rf-btn rf-btn-sm",
                                                    onclick: move |_| {
                                                        if let Some(resp_view) = resp_view.clone() {
                                                            save_response_example(
                                                                st_ex.clone(),
                                                                ep.id,
                                                                resp_view,
                                                                examples,
                                                            )
                                                        }
                                                    },
                                                    "保存为示例"
                                                }
                                            }
                                        }
                                    }
                                    if sending_visible {
                                        span { class: "rf-spin-sm" }
                                        span { class: "hint-inline", "发送中……" }
                                    }
                                }
                                if let Some(r) = resp {
                                    div { class: "resp-summary",
                                        label { "响应头" }
                                        for (k, v) in r.headers.clone() {
                                            div { class: "resp-hdr-row",
                                                code { "{k}" }
                                                span { "{v}" }
                                            }
                                        }
                                    }
                                    pre {
                                        class: if is_json_content_type(&r.content_type) { "resp-body-hl" } else { "" },
                                        {
                                            if is_json_content_type(&r.content_type) {
                                                highlight_json(resp_body_text.clone())
                                            } else {
                                                rsx! { {resp_body_text.clone()} }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // M5：历史抽屉（替代弹窗）。
                        if show_history_flag {
                            div {
                                class: "rf-drawer-backdrop",
                                onclick: move |_| {
                                    let mut sh = show_history;
                                    sh.set(false);
                                },
                            }
                            div {
                                class: "rf-drawer",
                                onclick: |e| { e.stop_propagation(); },
                                div { class: "rf-drawer-head",
                                    h3 { "请求历史" }
                                    button {
                                        class: "rf-tree-action",
                                        title: "关闭",
                                        onclick: move |_| {
                                            let mut sh = show_history;
                                            sh.set(false);
                                        },
                                        XIcon {}
                                    }
                                }
                                div { class: "rf-drawer-body",
                                    div { class: "history-list",
                                        for h in history_list.clone() {
                                            { history_item(&h, selected_history) }
                                        }
                                        if history_list.is_empty() {
                                            div { class: "empty", "暂无历史记录" }
                                        }
                                    }
                                    if let Some(h) = selected_history.peek().clone() {
                                        div { class: "history-detail",
                                            pre {
                                                "{prettify_summary(&h.response_summary_json)}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // M13：代码生成弹窗。
                        if codegen_open_flag {
                            RFModal {
                                on_close: move |_| {
                                    let mut co = codegen_open;
                                    co.set(false);
                                },
                                div { class: "row rf-mb-2",
                                        span { class: "kv-title", "生成客户端代码" }
                                        div { class: "spacer" }
                                        Dropdown {
                                            class: "rf-dd-lang",
                                            options: vec![
                                                ("curl".into(), "curl".into()),
                                                ("python".into(), "Python (requests)".into()),
                                                ("js".into(), "JavaScript (fetch)".into()),
                                                ("go".into(), "Go (net/http)".into()),
                                                ("java".into(), "Java (OkHttp)".into()),
                                                ("php".into(), "PHP (cURL)".into()),
                                            ],
                                            selected: codegen_lang_str.clone(),
                                            on_select: move |v: String| {
                                                let lang = Lang::from_str_cn(&v).unwrap_or(Lang::Curl);
                                                let mut cl = codegen_lang;
                                                cl.set(v);
                                                let mut cc = codegen_code;
                                                if let Some(ep) = draft.peek().clone() {
                                                    if let Some(code) = build_codegen_code(&st_cg, &ep, lang) {
                                                        cc.set(code);
                                                    }
                                                }
                                            },
                                        }
                                        button {
                                            class: "rf-btn rf-btn-sm",
                                            title: "复制到剪贴板",
                                            onclick: move |_| {
                                                let code = codegen_code.peek().clone();
                                                if code.is_empty() {
                                                    st_cg_copy.toast_info("暂无生成的代码");
                                                    return;
                                                }
                                                copy_text(&code);
                                                st_cg_copy.toast_success("已复制到剪贴板");
                                            },
                                            CopyIcon {}
                                            "复制"
                                        }
                                    }
                                    pre {
                                        class: "codegen-out",
                                        "{codegen_code_str}"
                                    }
                                }
                            }
                        // M18：未落库接口保存命名弹窗。
                        if pending_save_ep.is_some() {
                            RFModal {
                                on_close: move |_| {
                                    let mut ps = pending_save;
                                    ps.set(None);
                                    save_name_synced.set(None);
                                },
                                div { class: "kv-title", "保存接口" }
                                div { class: "rf-mb-2 hint-inline", "该接口尚未保存，请输入接口名称" }
                                input {
                                    class: "rf-input",
                                    placeholder: "接口名称",
                                    value: "{save_name_str}",
                                    oninput: move |e| {
                                        let mut sn = save_name;
                                        sn.set(e.data().value());
                                    },
                                }
                                div { class: "row rf-mb-2",
                                    div { class: "spacer" }
                                    button {
                                        class: "rf-btn rf-btn-sm",
                                        onclick: move |_| {
                                            let mut ps = pending_save;
                                            ps.set(None);
                                            save_name_synced.set(None);
                                        },
                                        "取消"
                                    }
                                    button {
                                        class: "rf-btn rf-btn-primary rf-btn-sm",
                                        onclick: move |_| {
                                            let name = save_name.peek().clone();
                                            // 先同步本地草稿名，保存后脏标记才能正确清除。
                                            let local_draft = draft.peek().clone();
                                            if let (Some(mut ep), Some(pending_ep)) =
                                                (local_draft, pending_save_ep.clone())
                                            {
                                                if ep.id == pending_ep.id {
                                                    ep.name = name.clone();
                                                    draft.set(Some(ep));
                                                }
                                            }
                                            st_confirm_save.confirm_save_name(name);
                                        },
                                        "保存"
                                    }
                                }
                            }
                        }
                        // M17：导入 cURL 弹窗。
                        if curl_open_flag {
                            RFModal {
                                on_close: move |_| {
                                    let mut co = curl_open;
                                    co.set(false);
                                },
                                div { class: "kv-title", "从 cURL 命令导入" }
                                    div {
                                        class: "hint",
                                        "粘贴浏览器开发者工具「Copy as cURL」复制的命令，自动识别方法、URL、请求头、请求体与 Basic 认证。",
                                    }
                                    textarea {
                                        class: "rf-textarea curl-input",
                                        rows: "10",
                                        placeholder: "例如：\ncurl -X POST https://api.example.com/users \\\n  -H \"Content-Type: application/json\" \\\n  -u user:pass \\\n  -d \"{{\"name\":\"test\"}}\"",
                                        value: "{curl_input}",
                                        oninput: move |e| {
                                            let v = e.data().value();
                                            let mut ci = curl_input;
                                            ci.set(v);
                                        },
                                    }
                                    div { class: "rf-modal-actions",
                                        button { class: "rf-btn", onclick: move |_| {
                                            let mut co = curl_open;
                                            co.set(false);
                                        }, "取消" }
                                        button { class: "rf-btn rf-btn-primary", onclick: move |_| import_curl(), "解析并导入" }
                                    }
                                }
                            }
                        }
                    if let Some((info, _)) = confirm.read().as_ref() {
                        ConfirmDialog {
                            info: Some(info.clone()),
                            on_confirm: move |_| {
                                if let Some((_, target)) = confirm.peek().as_ref() {
                                    match *target {
                                        DeleteTarget::TestHistory { run_id, project_id } => {
                                            delete_test_history_entry(
                                                st.clone(),
                                                run_id,
                                                project_id,
                                                test_history,
                                            )
                                        }
                                        DeleteTarget::ResponseExample { example_id, endpoint_id } => {
                                            delete_example(st.clone(), example_id, endpoint_id, examples)
                                        }
                                    }
                                }
                                let mut c = confirm;
                                c.set(None);
                            },
                            on_cancel: move |_| {
                                let mut c = confirm;
                                c.set(None);
                            },
                        }
                    }
                    {loading_overlay}
                }
    } else {
        rsx! {
            if loading_flag {
                div { class: "rf-empty",
                    div { class: "rf-spinner" }
                    "正在加载项目数据…"
                }
            } else {
                EmptyState {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_mode_switch_preserves_raw() {
        let cur = BodySpec::Json {
            raw: "{\"a\":1}".into(),
        };
        let text = switch_body_mode(&cur, "text");
        assert!(matches!(text, BodySpec::Text { raw } if raw == "{\"a\":1}"));
        let none = switch_body_mode(&cur, "none");
        assert!(matches!(none, BodySpec::None));
        let ue = switch_body_mode(&cur, "urlencoded");
        assert!(matches!(ue, BodySpec::UrlEncoded { fields } if fields.is_empty()));
    }

    #[test]
    fn body_mode_switch_keeps_raw_across_none() {
        let mut raw_keep = String::new();
        let mut fields_keep: Vec<KeyValue> = Vec::new();
        let cur = BodySpec::Json {
            raw: "{\"a\":1}".into(),
        };
        let none = switch_body_mode_keep(&cur, "none", &mut raw_keep, &mut fields_keep);
        assert!(matches!(none, BodySpec::None));
        let back = switch_body_mode_keep(&none, "json", &mut raw_keep, &mut fields_keep);
        assert!(matches!(back, BodySpec::Json { raw } if raw == "{\"a\":1}"));
        let text = switch_body_mode_keep(&cur, "text", &mut raw_keep, &mut fields_keep);
        assert!(matches!(&text, BodySpec::Text { raw } if raw == "{\"a\":1}"));
        let none2 = switch_body_mode_keep(&text, "none", &mut raw_keep, &mut fields_keep);
        let text_back = switch_body_mode_keep(&none2, "text", &mut raw_keep, &mut fields_keep);
        assert!(matches!(text_back, BodySpec::Text { raw } if raw == "{\"a\":1}"));
    }

    #[test]
    fn body_mode_switch_keeps_fields_across_none() {
        let mut raw_keep = String::new();
        let mut fields_keep: Vec<KeyValue> = Vec::new();
        let cur = BodySpec::UrlEncoded {
            fields: vec![KeyValue {
                key: "name".into(),
                value: "fox".into(),
                enabled: true,
                description: String::new(),
            }],
        };
        let none = switch_body_mode_keep(&cur, "none", &mut raw_keep, &mut fields_keep);
        let back = switch_body_mode_keep(&none, "urlencoded", &mut raw_keep, &mut fields_keep);
        assert!(matches!(
            back,
            BodySpec::UrlEncoded { fields } if fields.len() == 1 && fields[0].key == "name" && fields[0].value == "fox"
        ));
    }

    #[test]
    fn auth_switch_preserves_fields() {
        let cur = AuthSpec::Basic {
            username: "u".into(),
            password: "p".into(),
        };
        let back = switch_auth(&cur, "basic");
        assert!(matches!(
            back,
            AuthSpec::Basic { username, password } if username == "u" && password == "p"
        ));
        let bearer = switch_auth(&cur, "bearer");
        assert!(matches!(bearer, AuthSpec::Bearer { token } if token.is_empty()));
        let none = switch_auth(&cur, "none");
        assert!(matches!(none, AuthSpec::None));
    }

    #[test]
    fn format_json_valid_and_invalid() {
        let pretty = format_json("{\"b\":2,\"a\":1}").unwrap();
        let compact = pretty.replace(['\n', ' '], "");
        assert_eq!(compact, "{\"a\":1,\"b\":2}");
        assert!(format_json("{oops").is_err());
    }

    #[test]
    fn apply_curl_overwrites_draft_fields() {
        let mut ep = ep_with_path("/old");
        ep.name = "新建接口".into();
        let parsed = parse_curl(
            "curl -X POST -H \"Content-Type: application/json\" -u u:p \
             -d '{\"a\":1}' https://api.example.com/users",
        )
        .unwrap();
        apply_curl(&mut ep, &parsed);
        assert_eq!(ep.method, HttpMethod::POST);
        assert_eq!(ep.path, "https://api.example.com/users");
        assert_eq!(ep.request.headers.len(), 1);
        assert!(matches!(ep.request.auth, AuthSpec::Basic { .. }));
        assert!(matches!(
            ep.request.body,
            BodySpec::Json { raw } if raw.trim() == "{\"a\":1}"
        ));
        assert_eq!(ep.name, "users");
    }

    #[test]
    fn apply_curl_keeps_custom_name() {
        let mut ep = ep_with_path("/old");
        ep.name = "我的接口".into();
        let parsed = parse_curl("curl https://api.example.com/users").unwrap();
        apply_curl(&mut ep, &parsed);
        assert_eq!(ep.name, "我的接口");
    }

    fn ep_with_path(path: &str) -> Endpoint {
        let now = Utc::now();
        Endpoint {
            id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
            folder_id: None,
            name: "测试接口".into(),
            method: HttpMethod::GET,
            path: path.to_string(),
            description: String::new(),
            status: EndpointStatus::Developing,
            sort_order: 0,
            request: RequestSpec::default(),
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn render_request_keeps_absolute_url_without_base() {
        let ep = ep_with_path("https://httpbin.org/get");
        let (url, _) = render_request(&ep, &HashMap::new());
        assert_eq!(url, "https://httpbin.org/get");
    }

    #[test]
    fn render_request_absolute_url_wins_over_base_url() {
        let ep = ep_with_path("https://httpbin.org/get");
        let vars = HashMap::from([(
            "base_url".to_string(),
            "https://other.example.com".to_string(),
        )]);
        let (url, _) = render_request(&ep, &vars);
        assert_eq!(url, "https://httpbin.org/get");
    }

    #[test]
    fn render_request_joins_relative_path_with_base_url() {
        let ep = ep_with_path("/api/users");
        let vars = HashMap::from([(
            "base_url".to_string(),
            "https://api.example.com".to_string(),
        )]);
        let (url, _) = render_request(&ep, &vars);
        assert_eq!(url, "https://api.example.com/api/users");
    }

    #[test]
    fn render_request_missing_base_url_yields_relative_url() {
        let ep = ep_with_path("/api/users");
        let (url, _) = render_request(&ep, &HashMap::new());
        assert_eq!(url, "/api/users");
        assert!(!is_absolute_url(&url));
    }

    #[test]
    fn render_request_resolves_variables_in_path() {
        let mut ep = ep_with_path("/users/{id}");
        ep.request.path_variables = vec![KeyValue::new("id", "42")];
        let mut vars = HashMap::new();
        vars.insert("base_url".to_string(), "http://localhost:8080".to_string());
        let (url, _) = render_request(&ep, &vars);
        assert_eq!(url, "http://localhost:8080/users/42");
    }
}
