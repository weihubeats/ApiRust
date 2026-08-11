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
///
/// 单次扫描实现：一次遍历输入，定位全部 `{{...}}` token，
/// `with_capacity` 预分配后直接拼接最终结果；token 值内嵌套的
/// 变量逐层递归解析（上限 `max_depth`），不再整串反复扫描与重建。
pub fn resolve_variables_with(
    input: &str,
    vars: &VariableMap,
    max_depth: usize,
    options: ResolveOptions,
) -> String {
    let mut out = String::with_capacity(input.len() + 16);
    resolve_into(&mut out, input, vars, max_depth, options);
    out
}

/// 单次扫描：将 `s` 中所有可解析 token 替换后追加到 `out`。
///
/// - `depth` 为剩余解析层数，为 0 时整体按字面量输出；
/// - 未知变量、空 token、未闭合的 `{{` 均原样保留；
/// - 按字节扫描定位 `{` / `}`：二者均为 ASCII，不会与多字节 UTF-8 序列冲突。
fn resolve_into(
    out: &mut String,
    s: &str,
    vars: &VariableMap,
    depth: usize,
    options: ResolveOptions,
) {
    if depth == 0 {
        out.push_str(s);
        return;
    }
    let bytes = s.as_bytes();
    let mut copied = 0; // 已拷入 out 的字面量区间终点（相对 s 起点的字节偏移）
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'{' && bytes[i + 1] == b'{' {
            // 查找闭合的 }}。
            let mut close = i + 2;
            while close + 1 < bytes.len() {
                if bytes[close] == b'}' && bytes[close + 1] == b'}' {
                    break;
                }
                close += 1;
            }
            if close + 1 >= bytes.len() {
                break; // 未闭合：其余内容一律按字面量处理
            }
            out.push_str(&s[copied..i]);
            let name = s[i + 2..close].trim();
            if name.is_empty() {
                out.push_str(&s[i..close + 2]);
            } else if let Some(value) = lookup(name, vars, options) {
                // 值内可能仍有 {{...}}：递归解析，消耗一层深度。
                resolve_into(out, &value, vars, depth - 1, options);
            } else {
                out.push_str(&s[i..close + 2]);
            }
            copied = close + 2;
            i = close + 2;
        } else {
            i += 1;
        }
    }
    out.push_str(&s[copied..]);
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

    #[test]
    fn many_tokens_resolved_in_one_pass() {
        let v = vars(&[("id", "42")]);
        let input = "{{id}},".repeat(1000);
        let out = resolve_variables(&input, &v);
        assert_eq!(out, "42,".repeat(1000));
        assert!(!out.contains("{{"));
    }

    #[test]
    fn unicode_literals_preserved() {
        let v = vars(&[("name", "小狐狸"), ("id", "9")]);
        assert_eq!(
            resolve_variables("你好，{{name}}！#{{id}}号", &v),
            "你好，小狐狸！#9号"
        );
    }

    #[test]
    fn unclosed_token_kept_as_literal() {
        let v = vars(&[("a", "1")]);
        assert_eq!(
            resolve_variables("{{a}} and {{unclosed", &v),
            "1 and {{unclosed"
        );
    }
}
