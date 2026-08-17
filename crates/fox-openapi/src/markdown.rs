//! Markdown 文档导出（M10）。

use std::collections::HashMap;

use chrono::Utc;
use fox_core::model::{AuthSpec, BodySpec, Endpoint, KeyValue, ResponseExample};

/// 将一组接口导出为 Markdown 文档。
pub fn export_markdown(
    project_name: &str,
    endpoints: &[Endpoint],
    examples_by_endpoint: &HashMap<uuid::Uuid, Vec<ResponseExample>>,
) -> String {
    let mut out = String::with_capacity(4096);
    out.push_str(&format!("# {project_name}\n"));
    out.push_str(&format!(
        "- 导出时间：{}\n\n",
        Utc::now().format("%Y-%m-%d %H:%M:%S")
    ));

    for (idx, ep) in endpoints.iter().enumerate() {
        out.push_str(&format!(
            "## {}. {}（{} {}）\n\n",
            idx + 1,
            ep.name,
            ep.method.as_str(),
            ep.path
        ));
        if !ep.description.is_empty() {
            out.push_str("### 描述\n\n");
            out.push_str(&ep.description);
            out.push_str("\n\n");
        }
        out.push_str("### 参数\n\n");
        out.push_str("| 名称 | 值 | 描述 | 启用 |\n|---|---|---|---|\n");
        for kv in &ep.request.params {
            out.push_str(&kv_line(kv));
        }
        out.push('\n');

        out.push_str("### 请求头\n\n");
        out.push_str("| 名称 | 值 | 描述 | 启用 |\n|---|---|---|---|\n");
        for kv in &ep.request.headers {
            out.push_str(&kv_line(kv));
        }
        out.push('\n');

        match &ep.request.auth {
            AuthSpec::None => {}
            AuthSpec::Bearer { token } => {
                out.push_str("### 认证\n\n");
                out.push_str(&format!("- Bearer Token：`{token}`\n\n"));
            }
            AuthSpec::Basic { username, password } => {
                out.push_str("### 认证\n\n");
                out.push_str(&format!("- Basic Auth：`{username}` / `{password}`\n\n"));
            }
            AuthSpec::ApiKey {
                key,
                value,
                location,
            } => {
                out.push_str("### 认证\n\n");
                out.push_str(&format!(
                    "- API Key：`{key}`（{}内）值 `{value}`\n\n",
                    if *location == fox_core::model::ApiKeyLocation::Header {
                        "Header"
                    } else {
                        "Query"
                    }
                ));
            }
            AuthSpec::OAuth2 { .. } => {
                out.push_str("### 认证\n\n");
                out.push_str("- OAuth 2.0：授权码流（Bearer Token）\n\n");
            }
        }

        let body_text = match &ep.request.body {
            BodySpec::Json { raw } | BodySpec::Text { raw } => Some(raw.as_str()),
            BodySpec::UrlEncoded { fields } => {
                if fields.is_empty() {
                    None
                } else {
                    let lines: Vec<String> = fields
                        .iter()
                        .filter(|kv| kv.enabled)
                        .map(|kv| format!("{}={}", kv.key, kv.value))
                        .collect();
                    let joined = format!(
                        "表单（application/x-www-form-urlencoded）：\n\n```\n{}\n```",
                        lines.join("\n")
                    );
                    out.push_str("### 请求体\n\n");
                    out.push_str(&joined);
                    out.push_str("\n\n");
                    None
                }
            }
            BodySpec::None => None,
            BodySpec::Multipart { .. } => None,
            BodySpec::Binary { .. } => None,
            BodySpec::GraphQL { spec } => {
                out.push_str("### 请求体（GraphQL）\n\n");
                out.push_str(&format!("```graphql\n{}\n```\n\n", spec.query));
                let trimmed = spec.variables.trim();
                if !trimmed.is_empty() && trimmed != "{}" {
                    out.push_str(&format!("**变量：**\n\n```json\n{trimmed}\n```\n\n"));
                }
                None
            }
        };
        if let Some(raw) = body_text {
            out.push_str("### 请求体\n\n```json\n");
            out.push_str(raw);
            out.push_str("\n```\n\n");
        }

        out.push_str("### 响应示例\n\n");
        let examples = examples_by_endpoint.get(&ep.id);
        match examples {
            Some(list) if !list.is_empty() => {
                for ex in list {
                    out.push_str(&format!(
                        "**{status} - {name}**\n\n",
                        status = ex.status,
                        name = ex.name
                    ));
                    if !ex.content_type.is_empty() {
                        out.push_str(&format!("- Content-Type：`{}`\n\n", ex.content_type));
                    }
                    out.push_str("```json\n");
                    out.push_str(&ex.body);
                    out.push_str("\n```\n\n");
                }
            }
            _ => {
                out.push_str("（无）\n\n");
            }
        }
    }
    out
}

fn kv_line(kv: &KeyValue) -> String {
    let desc = if kv.description.is_empty() {
        "-".to_string()
    } else {
        kv.description.clone()
    };
    format!(
        "| {} | {} | {} | {} |\n",
        kv.key.replace('|', "\\|"),
        kv.value.replace('|', "\\|"),
        desc.replace('|', "\\|"),
        if kv.enabled { "是" } else { "否" }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use fox_core::model::{EndpointStatus, HttpMethod, RequestSpec};
    use uuid::Uuid;

    fn ep(name: &str) -> Endpoint {
        let mut request = RequestSpec::default();
        request.params.push(KeyValue::new("id", "1"));
        request.headers.push(KeyValue::new("X-Token", "abc"));
        request.body = BodySpec::Json {
            raw: "{\"a\":1}".into(),
        };
        Endpoint {
            id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
            folder_id: None,
            name: name.to_string(),
            method: HttpMethod::GET,
            path: "/api/users/{id}".into(),
            description: "获取用户".into(),
            status: EndpointStatus::Developing,
            sort_order: 0,
            request,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn markdown_basic_structure() {
        let ep = ep("用户列表");
        let mut map = HashMap::new();
        let r = ResponseExample {
            id: Uuid::new_v4(),
            endpoint_id: ep.id,
            name: "成功".into(),
            status: 200,
            headers: HashMap::new(),
            body: r#"{"id":1}"#.into(),
            content_type: "application/json".into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        map.insert(ep.id, vec![r]);
        let md = export_markdown("测试项目", &[ep], &map);
        assert!(md.starts_with("# 测试项目\n"));
        assert!(md.contains("## 1. 用户列表（GET /api/users/{id}）"));
        assert!(md.contains("### 参数"));
        assert!(md.contains("| id | 1 | - | 是 |"));
        assert!(md.contains("X-Token"));
        assert!(md.contains("```json"));
        assert!(md.contains("200 - 成功"));
    }

    #[test]
    fn markdown_without_examples() {
        let ep = ep("空示例");
        let md = export_markdown("P", &[ep], &HashMap::new());
        assert!(md.contains("（无）"));
    }
}
