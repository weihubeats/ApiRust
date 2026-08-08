//! Postman Collection v2.1 导入（M12）。
//!
//! 支持 item 目录递归（→ 文件夹）、url 对象 / 字符串、
//! header / query（disabled → 停用）、body（raw / urlencoded /
//! formdata）、collection / request 级 auth（basic / bearer / apikey）、
//! response 数组 → 响应示例。

use std::collections::HashMap;

use fox_core::model::{
    ApiKeyLocation, AuthSpec, BodySpec, HttpMethod, KeyValue, MultipartField, MultipartValueType,
    RequestSpec,
};
use fox_core::AppError;
use serde_json::Value;

use crate::import::{parse_value, ImportedEndpoint, ImportedExample};

/// 导入 Postman 集合 v2.1，返回待落库的接口列表。
pub fn import_postman(text: &str) -> Result<Vec<ImportedEndpoint>, AppError> {
    let doc = parse_value(text)?;
    let root = doc
        .as_object()
        .ok_or_else(|| AppError::Validation("Postman 集合根节点不是对象".to_string()))?;
    if !root.contains_key("item") {
        return Err(AppError::Validation(
            "不是有效的 Postman 集合（缺少 item 列表）".to_string(),
        ));
    }
    let schema = root
        .get("info")
        .and_then(|i| i.get("schema"))
        .and_then(|s| s.as_str())
        .unwrap_or_default();
    if !schema.contains("postman") {
        return Err(AppError::Validation(
            "不是有效的 Postman 集合 v2.1（info.schema 缺失）".to_string(),
        ));
    }

    let collection_auth = parse_auth(root.get("auth"));
    let mut out = Vec::new();
    let items = root
        .get("item")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    for item in items {
        walk_items(item, None, &collection_auth, &mut out);
    }
    Ok(out)
}

fn walk_items(
    item: Value,
    folder_hint: Option<String>,
    collection_auth: &Option<AuthSpec>,
    out: &mut Vec<ImportedEndpoint>,
) {
    let Some(obj) = item.as_object() else {
        return;
    };
    let name = obj
        .get("name")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    if let Some(children) = obj.get("item").and_then(|v| v.as_array()) {
        // 分组 → 文件夹提示（一级分组即可）。
        let group_hint = folder_hint.clone().or_else(|| {
            if !name.is_empty() {
                Some(name.clone())
            } else {
                None
            }
        });
        for child in children.clone() {
            walk_items(child, group_hint.clone(), collection_auth, out);
        }
        return;
    }
    let Some(request) = obj.get("request") else {
        return;
    };
    let req_name = obj
        .get("name")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    if let Some(ep) = map_request(&req_name, request, folder_hint, collection_auth, obj) {
        out.push(ep);
    }
}

fn map_request(
    item_name: &str,
    request: &Value,
    folder_hint: Option<String>,
    collection_auth: &Option<AuthSpec>,
    parent: &serde_json::Map<String, Value>,
) -> Option<ImportedEndpoint> {
    let req = request.as_object()?;
    let method = req
        .get("method")
        .and_then(|x| x.as_str())
        .unwrap_or("GET")
        .parse::<HttpMethod>()
        .ok()?;

    // URL：字符串或对象。
    let url_val = req.get("url");
    let mut params = Vec::new();
    let mut raw_path = String::new();
    match url_val {
        Some(Value::String(s)) => raw_path = s.clone(),
        Some(Value::Object(url_obj)) => {
            raw_path = url_obj
                .get("raw")
                .and_then(|x| x.as_str())
                .map(|x| x.to_string())
                .unwrap_or_default();
            if let Some(query) = url_obj.get("query").and_then(|v| v.as_array()) {
                for q in query {
                    let Some(q) = q.as_object() else {
                        continue;
                    };
                    params.push(KeyValue {
                        key: q
                            .get("key")
                            .and_then(|x| x.as_str())
                            .unwrap_or("")
                            .to_string(),
                        value: q
                            .get("value")
                            .and_then(|x| x.as_str())
                            .unwrap_or("")
                            .to_string(),
                        enabled: !q.get("disabled").and_then(|x| x.as_bool()).unwrap_or(false),
                        description: q
                            .get("description")
                            .and_then(|x| x.as_str())
                            .unwrap_or("")
                            .to_string(),
                    });
                }
            }
        }
        _ => {}
    }
    if raw_path.is_empty() {
        return None;
    }

    let mut headers = Vec::new();
    if let Some(hs) = req.get("header").and_then(|v| v.as_array()) {
        for h in hs {
            let Some(h) = h.as_object() else {
                continue;
            };
            headers.push(KeyValue {
                key: h
                    .get("key")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string(),
                value: h
                    .get("value")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string(),
                enabled: !h.get("disabled").and_then(|x| x.as_bool()).unwrap_or(false),
                description: h
                    .get("description")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string(),
            });
        }
    }

    let body = parse_body(req.get("body"));

    let auth = parse_auth(req.get("auth"))
        .or_else(|| collection_auth.clone())
        .unwrap_or(AuthSpec::None);

    let description = req
        .get("description")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();

    // 响应示例。
    let mut examples = Vec::new();
    if let Some(responses) = parent.get("response").and_then(|v| v.as_array()) {
        for r in responses {
            let Some(r) = r.as_object() else {
                continue;
            };
            let status = r
                .get("code")
                .and_then(|x| x.as_u64())
                .map(|x| x as u16)
                .unwrap_or(200);
            let content_type = r
                .get("header")
                .and_then(|v| v.as_array())
                .and_then(|arr| {
                    arr.iter().find_map(|h| {
                        let h = h.as_object()?;
                        let k = h.get("key").and_then(|x| x.as_str())?;
                        if k.eq_ignore_ascii_case("content-type") {
                            h.get("value")
                                .and_then(|x| x.as_str())
                                .map(|x| x.to_string())
                        } else {
                            None
                        }
                    })
                })
                .or_else(|| {
                    r.get("_postman_previewlanguage")
                        .and_then(|x| x.as_str())
                        .map(|x| {
                            if x.contains("html") {
                                "text/html".to_string()
                            } else {
                                "application/json".to_string()
                            }
                        })
                })
                .unwrap_or_else(|| "application/json".to_string());
            let name = r
                .get("name")
                .and_then(|x| x.as_str())
                .map(|x| x.to_string())
                .unwrap_or_else(|| format!("响应 {status}"));
            let body = r
                .get("body")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            examples.push(ImportedExample {
                name,
                status,
                content_type,
                headers: HashMap::new(),
                body,
            });
        }
    }

    let path = raw_path
        .trim_start_matches("http://")
        .trim_start_matches("https://");
    let slash = path.find('/');
    let path = match slash {
        Some(i) => &path[i..],
        None => "/",
    };
    let path = match path.find(['?', '#']) {
        Some(i) => &path[..i],
        None => path,
    };
    if path.is_empty() {
        return None;
    }

    Some(ImportedEndpoint {
        name: if item_name.is_empty() {
            format!("{method} {path}")
        } else {
            item_name.to_string()
        },
        method,
        path: path.to_string(),
        description,
        request: RequestSpec {
            params,
            headers,
            path_variables: Vec::new(),
            auth,
            body,
            ..Default::default()
        },
        examples,
        folder_hint,
    })
}

fn parse_body(body: Option<&Value>) -> BodySpec {
    let Some(body) = body.and_then(|b| b.as_object()) else {
        return BodySpec::None;
    };
    let mode = body.get("mode").and_then(|x| x.as_str()).unwrap_or("");
    match mode {
        "raw" => {
            let raw = body
                .get("raw")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let lang = body
                .get("options")
                .and_then(|o| o.get("raw"))
                .and_then(|r| r.get("language"))
                .and_then(|l| l.as_str())
                .unwrap_or("json");
            if lang.contains("json") {
                BodySpec::Json { raw }
            } else if lang == "xml" || lang == "html" || lang == "text" {
                BodySpec::Text { raw }
            } else {
                BodySpec::Json { raw }
            }
        }
        "urlencoded" => {
            let fields = body
                .get("urlencoded")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|f| {
                            let f = f.as_object()?;
                            Some(KeyValue {
                                key: f
                                    .get("key")
                                    .and_then(|x| x.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                value: f
                                    .get("value")
                                    .and_then(|x| x.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                enabled: !f
                                    .get("disabled")
                                    .and_then(|x| x.as_bool())
                                    .unwrap_or(false),
                                description: f
                                    .get("description")
                                    .and_then(|x| x.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            BodySpec::UrlEncoded { fields }
        }
        "formdata" => {
            let fields = body
                .get("formdata")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|f| {
                            let f = f.as_object()?;
                            let value_type =
                                if f.get("type").and_then(|x| x.as_str()) == Some("file") {
                                    MultipartValueType::FilePath
                                } else {
                                    MultipartValueType::Text
                                };
                            Some(MultipartField {
                                key: f
                                    .get("key")
                                    .and_then(|x| x.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                value: f
                                    .get("value")
                                    .and_then(|x| x.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                value_type,
                                enabled: !f
                                    .get("disabled")
                                    .and_then(|x| x.as_bool())
                                    .unwrap_or(false),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            BodySpec::Multipart { fields }
        }
        "file" => BodySpec::Text {
            raw: body
                .get("file")
                .and_then(|f| f.get("src"))
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
        },
        _ => BodySpec::None,
    }
}

fn parse_auth(auth: Option<&Value>) -> Option<AuthSpec> {
    let obj = auth?.as_object()?;
    let ty = obj.get("type").and_then(|x| x.as_str()).unwrap_or("noauth");
    let arr = obj.get(ty).and_then(|v| v.as_array());
    let cred = |name: &str| -> String {
        arr.and_then(|arr| {
            arr.iter().find_map(|k| {
                let k = k.as_object()?;
                if k.get("key").and_then(|x| x.as_str()) == Some(name) {
                    k.get("value")
                        .and_then(|x| x.as_str())
                        .map(|x| x.to_string())
                } else {
                    None
                }
            })
        })
        .unwrap_or_default()
    };
    match ty {
        "basic" => Some(AuthSpec::Basic {
            username: cred("username"),
            password: cred("password"),
        }),
        "bearer" => Some(AuthSpec::Bearer {
            token: cred("token"),
        }),
        "apikey" => Some(AuthSpec::ApiKey {
            key: cred("key"),
            value: cred("value"),
            location: if cred("in") == "header" {
                ApiKeyLocation::Header
            } else {
                ApiKeyLocation::Query
            },
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const COLLECTION: &str = r#"{
      "info": {
        "name": "Demo",
        "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
      },
      "auth": {
        "type": "bearer",
        "bearer": [{ "key": "token", "value": "abc123" }]
      },
      "item": [
        {
          "name": "用户管理",
          "item": [
            {
              "name": "获取用户",
              "request": {
                "method": "GET",
                "header": [{ "key": "Accept", "value": "application/json" }],
                "url": {
                  "raw": "https://api.example.com/users/{{uid}}?verbose=1&expand=0",
                  "host": ["api.example.com"],
                  "path": ["users", "{{uid}}"],
                  "query": [
                    { "key": "verbose", "value": "1" },
                    { "key": "expand", "value": "0", "disabled": true }
                  ]
                },
                "auth": {
                  "type": "apikey",
                  "apikey": [
                    { "key": "key", "value": "X-API-Key", "type": "string" },
                    { "key": "value", "value": "k1", "type": "string" },
                    { "key": "in", "value": "header", "type": "string" }
                  ]
                },
                "description": "获取用户信息"
              },
              "response": [
                {
                  "name": "成功",
                  "code": 200,
                  "header": [{ "key": "Content-Type", "value": "application/json" }],
                  "body": "{\"ok\":true}"
                }
              ]
            }
          ]
        },
        {
          "name": "登录",
          "request": {
            "method": "POST",
            "body": {
              "mode": "urlencoded",
              "urlencoded": [{ "key": "u", "value": "admin" }]
            },
            "url": "https://api.example.com/login"
          }
        }
      ]
    }"#;

    #[test]
    fn rejects_non_postman_docs() {
        assert!(import_postman(r#"{"item":[]}"#).is_err());
        assert!(import_postman("nonsense").is_err());
    }

    #[test]
    fn folder_recursion_and_url_mapping() {
        let list = import_postman(COLLECTION).unwrap();
        assert_eq!(list.len(), 2);

        let get = list.iter().find(|e| e.method.to_string() == "GET").unwrap();
        assert_eq!(get.folder_hint.as_deref(), Some("用户管理"));
        assert_eq!(get.path, "/users/{{uid}}");
        assert_eq!(get.name, "获取用户");
        assert_eq!(get.request.params.len(), 2);
        assert!(get.request.params[0].enabled);
        assert!(!get.request.params[1].enabled);
        assert_eq!(get.request.headers[0].key, "Accept");
        assert_eq!(get.description, "获取用户信息");
        assert_eq!(get.examples.len(), 1);
        assert_eq!(get.examples[0].status, 200);
        assert_eq!(get.examples[0].content_type, "application/json");
        assert!(get.examples[0].body.contains("ok"));
    }

    #[test]
    fn request_auth_overrides_collection() {
        let list = import_postman(COLLECTION).unwrap();
        let get = list.iter().find(|e| e.method.to_string() == "GET").unwrap();
        assert!(matches!(
            get.request.auth,
            fox_core::model::AuthSpec::ApiKey { .. }
        ));
        if let fox_core::model::AuthSpec::ApiKey {
            key,
            value,
            location,
        } = &get.request.auth
        {
            assert_eq!(key, "X-API-Key");
            assert_eq!(value, "k1");
            assert!(matches!(location, fox_core::model::ApiKeyLocation::Header));
        }
    }

    #[test]
    fn collection_auth_fallback_and_urlencoded_body() {
        let list = import_postman(COLLECTION).unwrap();
        let post = list
            .iter()
            .find(|e| e.method.to_string() == "POST")
            .unwrap();
        assert_eq!(post.folder_hint, None);
        assert!(matches!(
            post.request.auth,
            fox_core::model::AuthSpec::Bearer { .. }
        ));
        if let fox_core::model::AuthSpec::Bearer { token } = &post.request.auth {
            assert_eq!(token, "abc123");
        }
        let fox_core::model::BodySpec::UrlEncoded { fields } = &post.request.body else {
            panic!("期望 urlencoded body");
        };
        assert_eq!(fields[0].key, "u");
        assert_eq!(fields[0].value, "admin");
        assert_eq!(post.path, "/login");
    }
}
