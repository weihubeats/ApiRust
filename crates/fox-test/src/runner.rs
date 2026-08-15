//! 测试运行器（SPEC §17.2）：按配置执行单个接口的测试流程。

use std::collections::HashMap;

use fox_core::model::{Endpoint, RequestSpec};
use fox_http::client::{send_request, HttpResponseData};
use serde_json::Value;
use uuid::Uuid;

use crate::assert::{evaluate, Outcome};
use crate::config::TestSpec;
use crate::extract::extract_variables;

/// 单个接口的测试结果。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct EndpointResult {
    pub endpoint_id: Uuid,
    pub endpoint_name: String,
    pub method: String,
    pub path: String,
    pub ok: bool,
    pub status: Option<u16>,
    pub duration_ms: Option<u64>,
    /// 请求级错误（发送失败 / 配置错误）。
    pub request_error: Option<String>,
    /// 断言明细。
    pub outcomes: Vec<Outcome>,
}

impl EndpointResult {
    fn failed_fast(id: Uuid, ep: &Endpoint, reason: impl Into<String>) -> Self {
        EndpointResult {
            endpoint_id: id,
            endpoint_name: ep.name.clone(),
            method: ep.method.to_string(),
            path: ep.path.clone(),
            ok: false,
            status: None,
            duration_ms: None,
            request_error: Some(reason.into()),
            outcomes: Vec::new(),
        }
    }
}

/// 运行单个接口测试。
///
/// - `runtime_vars`：此前由 extract 得到的运行时变量，会被本轮 pre_request / extract
///   更新，并传递给后续接口（共享运行时上下文）。
///
/// 返回测试结果与原始响应。
pub async fn run_endpoint(
    ep: &Endpoint,
    url: &str,
    spec: &RequestSpec,
    runtime_vars: &mut HashMap<String, String>,
    timeout_ms: Option<u64>,
) -> (EndpointResult, Option<HttpResponseData>) {
    let config = match TestSpec::from_request_value(ep.request.tests.as_ref()) {
        Ok(c) => c,
        Err(reason) => {
            return (
                EndpointResult::failed_fast(ep.id, ep, format!("测试配置错误：{reason}")),
                None,
            );
        }
    };
    if config.is_empty() {
        return (
            EndpointResult {
                endpoint_id: ep.id,
                endpoint_name: ep.name.clone(),
                method: ep.method.to_string(),
                path: ep.path.clone(),
                ok: true,
                status: None,
                duration_ms: None,
                request_error: Some("无测试配置（跳过）".into()),
                outcomes: Vec::new(),
            },
            None,
        );
    }

    // 1. pre_request：设置变量（值先解析，支持 {{$timestamp}} 与前面提取的变量）。
    for p in &config.pre_request {
        let value = resolve_text(&p.value, runtime_vars);
        runtime_vars.insert(p.name.clone(), value);
    }

    // 2. 发送请求。
    let resp = match send_request(ep.method, url, spec, timeout_ms).await {
        Ok(r) => r,
        Err(e) => {
            return (
                EndpointResult::failed_fast(ep.id, ep, format!("请求失败：{}", e.user_message())),
                None,
            );
        }
    };

    // 3. 断言（expected 中的 {{变量}} 先按运行时上下文解析）。
    let body_value: Option<Value> = serde_json::from_str(&resp.body_text()).ok();
    let outcomes: Vec<Outcome> = config
        .assertions
        .iter()
        .map(|a| evaluate(&resolve(a, runtime_vars), &resp, body_value.as_ref()))
        .collect();
    let ok = outcomes.iter().all(|o| o.passed);

    // 4. 提取变量（供后续接口使用）。
    let extracted: HashMap<String, String> =
        extract_variables(&config.extract, &resp, body_value.as_ref());
    for (k, v) in extracted {
        runtime_vars.insert(k, v);
    }

    (
        EndpointResult {
            endpoint_id: ep.id,
            endpoint_name: ep.name.clone(),
            method: ep.method.to_string(),
            path: ep.path.clone(),
            ok,
            status: Some(resp.status),
            duration_ms: Some(resp.duration_ms),
            request_error: None,
            outcomes,
        },
        Some(resp),
    )
}

/// 按目录排序执行顺序：文件夹排序在前，文件夹内按接口 sort_order。
pub fn order_endpoints<'a>(
    endpoints: &'a [Endpoint],
    folder_order: &HashMap<Uuid, i64>,
) -> Vec<&'a Endpoint> {
    let mut pairs: Vec<(i64, i64, &Endpoint)> = endpoints
        .iter()
        .map(|ep| {
            let f = ep
                .folder_id
                .and_then(|id| folder_order.get(&id).copied())
                .unwrap_or(i64::MAX);
            (f, ep.sort_order, ep)
        })
        .collect();
    pairs.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then(a.1.cmp(&b.1))
            .then(a.2.name.cmp(&b.2.name))
    });
    pairs.into_iter().map(|(_, _, ep)| ep).collect()
}

fn resolve_text(input: &str, vars: &HashMap<String, String>) -> String {
    fox_core::resolve_variables(input, vars)
}

/// 断言 expected 里的 `{{变量}}` 先解析（仅在字符串期望时）。
fn resolve(
    a: &crate::config::AssertionSpec,
    vars: &HashMap<String, String>,
) -> crate::config::AssertionSpec {
    let mut a = a.clone();
    if let Some(serde_json::Value::String(s)) = &a.expected {
        if s.contains("{{") {
            a.expected = Some(serde_json::Value::String(resolve_text(s, vars)));
        }
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use fox_core::model::{Endpoint, EndpointStatus, HttpMethod, RequestSpec};

    fn ep(name: &str, path: &str, folder_id: Option<Uuid>, sort: i64) -> Endpoint {
        let request = RequestSpec {
            tests: Some(serde_json::json!({})),
            ..Default::default()
        };
        Endpoint {
            id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
            folder_id,
            name: name.to_string(),
            method: HttpMethod::GET,
            path: path.to_string(),
            description: String::new(),
            status: EndpointStatus::Developing,
            sort_order: sort,
            request,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn ordering_by_folder_then_sort() {
        let f1 = Uuid::new_v4();
        let f2 = Uuid::new_v4();
        let mut map = HashMap::new();
        map.insert(f1, 2);
        map.insert(f2, 1);
        let eps = vec![
            ep("c", "/c", Some(f1), 10),
            ep("a", "/a", None, 5),
            ep("b", "/b", Some(f2), 1),
            ep("d", "/d", Some(f1), 1),
        ];
        let ordered = order_endpoints(&eps, &map);
        let names: Vec<&str> = ordered.iter().map(|e| e.name.as_str()).collect();
        // f2 (order 1) 的 b 在前；f1（order 2）内按 sort_order：d(1) 在 c(10) 前；根目录最后。
        assert_eq!(names, vec!["b", "d", "c", "a"]);
    }
}
