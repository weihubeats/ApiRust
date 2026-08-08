//! 变量引擎：`{{name}}` 语法解析、内置变量、优先级合并。

use std::collections::HashMap;

use chrono::{SecondsFormat, Utc};
use rand::Rng;
use uuid::Uuid;

/// 变量表。
pub type VariableMap = HashMap<String, String>;

/// 最大递归深度。
pub const MAX_VARIABLE_DEPTH: usize = 10;

const BUILTIN_UUID: &str = "$uuid";
const BUILTIN_TIMESTAMP: &str = "$timestamp";
const BUILTIN_ISO_TIMESTAMP: &str = "$isoTimestamp";
const BUILTIN_RANDOM_INT: &str = "$randomInt";

/// 解析指内置变量是否可用。
#[derive(Debug, Clone, Copy)]
pub struct ResolveOptions {
    pub allow_builtin: bool,
}

impl Default for ResolveOptions {
    fn default() -> Self {
        ResolveOptions {
            allow_builtin: true,
        }
    }
}

/// 生成内置变量值；不认识的名字返回 None。
pub fn builtin_value(name: &str) -> Option<String> {
    match name {
        BUILTIN_UUID => Some(Uuid::new_v4().to_string()),
        BUILTIN_TIMESTAMP => Some(Utc::now().timestamp().to_string()),
        BUILTIN_ISO_TIMESTAMP => Some(Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)),
        BUILTIN_RANDOM_INT => Some(rand::thread_rng().gen_range(0..=1000).to_string()),
        _ => None,
    }
}

/// 按优先级合并三张变量表：运行时 > 环境 > 项目。
pub fn merge_variables(
    runtime: &VariableMap,
    environment: &VariableMap,
    project: &VariableMap,
) -> VariableMap {
    let mut merged = HashMap::new();
    for base in [project, environment, runtime] {
        for (k, v) in base {
            merged.insert(k.clone(), v.clone());
        }
    }
    merged
}

/// 解析文本中的 `{{name}}` 变量，最大递归 MAX_VARIABLE_DEPTH 层。
pub fn resolve_variables(input: &str, vars: &VariableMap) -> String {
    resolve_variables_with(input, vars, MAX_VARIABLE_DEPTH, ResolveOptions::default())
}

/// 解析文本中的 `{{name}}` 变量，指定最大递归深度。
pub fn resolve_variables_with(
    input: &str,
    vars: &VariableMap,
    max_depth: usize,
    options: ResolveOptions,
) -> String {
    let mut current = input.to_string();
    for _ in 0..max_depth {
        if !current.contains("{{") {
            break;
        }
        let rendered = render_once(&current, vars, options);
        if rendered == current {
            break;
        }
        current = rendered;
    }
    current
}

/// 单轮替换：找到第一个 `{{...}}` 并替换。
fn render_once(input: &str, vars: &VariableMap, options: ResolveOptions) -> String {
    fn find_token(s: &str) -> Option<(usize, usize, &str)> {
        let start = s.find("{{")?;
        let after = &s[start + 2..];
        match after.find("}}") {
            Some(rel) => {
                let end = start + 2 + rel;
                Some((start, end + 2, &s[start + 2..end]))
            }
            None => None,
        }
    }

    match find_token(input) {
        None => input.to_string(),
        Some((start, end, token)) => {
            let name = token.trim();
            let replacement = if name.is_empty() {
                None
            } else {
                lookup(name, vars, options)
            };
            let mut out = String::with_capacity(input.len() + 16);
            out.push_str(&input[..start]);
            match replacement {
                Some(value) => out.push_str(&value),
                None => out.push_str(&input[start..end]),
            }
            out.push_str(&input[end..]);
            out
        }
    }
}

/// 查找变量值。优先级：用户变量 > 内置变量。
fn lookup(name: &str, vars: &VariableMap, options: ResolveOptions) -> Option<String> {
    if let Some(value) = vars.get(name) {
        return Some(value.clone());
    }
    if options.allow_builtin {
        // 内置变量带 $ 前缀，避免与用户变量混淆。
        if let Some(value) = builtin_value(name) {
            return Some(value);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(&str, &str)]) -> VariableMap {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn basic_replace() {
        let v = vars(&[("base_url", "https://api.example.com"), ("id", "10")]);
        assert_eq!(
            resolve_variables("{{base_url}}/users/{{id}}", &v),
            "https://api.example.com/users/10"
        );
    }

    #[test]
    fn unknown_kept_as_is() {
        let v = VariableMap::new();
        assert_eq!(resolve_variables("{{missing}}/x", &v), "{{missing}}/x");
    }

    #[test]
    fn builtin_uuid() {
        let v = VariableMap::new();
        let out = resolve_variables("{{$uuid}}", &v);
        assert!(Uuid::parse_str(&out).is_ok());
    }

    #[test]
    fn builtin_iso_timestamp() {
        let v = VariableMap::new();
        let out = resolve_variables("{{$isoTimestamp}}", &v);
        assert!(chrono::DateTime::parse_from_rfc3339(&out).is_ok());
    }

    #[test]
    fn builtin_random_int_range() {
        let v = VariableMap::new();
        let out = resolve_variables("{{$randomInt}}", &v);
        let n: u64 = out.parse().unwrap();
        assert!((0..=1000).contains(&n));
    }

    #[test]
    fn user_var_overrides_builtin() {
        let v = vars(&[("$uuid", "custom")]);
        assert_eq!(resolve_variables("{{$uuid}}", &v), "custom");
    }

    #[test]
    fn nested_variables_recursion() {
        let v = vars(&[("a", "{{b}}"), ("b", "{{c}}"), ("c", "end")]);
        assert_eq!(resolve_variables("{{a}}", &v), "end");
    }

    #[test]
    fn nested_recursion_depth_capped() {
        let v = vars(&[("v0", "{{v1}}")]);
        let mut map = v;
        for i in 1..20 {
            map.insert(format!("v{i}"), format!("{{{{v{}}}}}", i + 1));
        }
        map.insert("v20".to_string(), "stop".to_string());
        // 超过 10 层后不再递归，保持未解析形态。
        let out = resolve_variables("{{v0}}", &map);
        assert!(out.len() > 2);
        assert!(out.contains("{{"));
    }

    #[test]
    fn merge_variables_priority() {
        let project = vars(&[("a", "project"), ("b", "project")]);
        let env = vars(&[("b", "env"), ("c", "env")]);
        let runtime = vars(&[("c", "runtime"), ("d", "runtime")]);
        let merged = merge_variables(&runtime, &env, &project);
        assert_eq!(merged["a"], "project");
        assert_eq!(merged["b"], "env");
        assert_eq!(merged["c"], "runtime");
        assert_eq!(merged["d"], "runtime");
    }

    #[test]
    fn empty_token_left_alone() {
        let v = VariableMap::new();
        assert_eq!(resolve_variables("a{{}}b", &v), "a{{}}b");
    }
}
