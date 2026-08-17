//! 共享文本工具：转义与编码。
//!
//! 所有生成器共用同一套「防错」原语，保证 Header / Query / Body 中的
//! 双引号、换行、反斜杠等特殊字符不会破坏目标语言语法：
//! - `encode_component`：URL 表单编码（RFC 3986 Component）
//! - `sq` / `dq`：单引号 / 双引号字符串字面量转义
//! - `pretty_json`：JSON 美化（解析失败原文回退，生成代码永不报错）
//! - `build_url` / `merged_headers`：查询参数拼接与请求头合并

use std::collections::HashMap;

use base64::Engine;

use crate::model::{ApiDefinition, AuthInfo};

/// RFC 3986 表单编码（Component 规则）。
pub(crate) fn encode_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    const HEX: &[u8] = b"0123456789ABCDEF";
    for b in s.as_bytes() {
        if b.is_ascii_alphanumeric() || *b == b'-' || *b == b'_' || *b == b'.' || *b == b'~' {
            out.push(*b as char);
        } else {
            out.push('%');
            out.push(HEX[(b >> 4) as usize] as char);
            out.push(HEX[(b & 0xF) as usize] as char);
        }
    }
    out
}

/// 转义单引号（sh / JS 单引号字符串）。
pub(crate) fn sq(s: &str) -> String {
    s.replace('\'', "'\\''")
}

/// 转义双引号字符串常量里的内容（Python / Java / Go / JS / PHP 通用）。
///
/// `"` → `\"`，`\` → `\\`，换行/回车/制表符转为转义序列，
/// 其余字符原样保留——保证嵌入任意文本都不会破坏目标语言字符串语法。
pub(crate) fn dq(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

/// 解析 JSON 并输出可读（pretty）格式；非法 JSON 原文回退。
///
/// 保证「美化」步骤自身不失败：输入不可解析时不做任何改动，生成代码依然合法。
pub(crate) fn pretty_json(raw: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(raw) {
        Ok(value) => serde_json::to_string_pretty(&value).unwrap_or_else(|_| raw.to_string()),
        Err(_) => raw.to_string(),
    }
}

/// 拼接查询参数到 URL（已含 `?` 时追加 `&`），键排序保证输出确定性。
pub(crate) fn build_url(base: &str, query: &HashMap<String, String>) -> String {
    if query.is_empty() {
        return base.to_string();
    }
    let mut keys: Vec<&String> = query.keys().collect();
    keys.sort();
    let joined: Vec<String> = keys
        .iter()
        .map(|key| {
            format!(
                "{}={}",
                encode_component(key),
                encode_component(&query[*key])
            )
        })
        .collect();
    let params = joined.join("&");
    if base.contains('?') {
        format!("{base}&{params}")
    } else {
        format!("{base}?{params}")
    }
}

/// 合并请求头与认证信息：auth 先生成，显式头后写覆盖；大小写不敏感去重；
/// 最终按键排序保证各语言输出稳定可复现。
pub(crate) fn merged_headers(api: &ApiDefinition) -> Vec<(String, String)> {
    let mut merged: Vec<(String, String)> = auth_headers(&api.auth);
    for (key, value) in &api.headers {
        if let Some(existing) = merged
            .iter_mut()
            .find(|(existing_key, _)| existing_key.eq_ignore_ascii_case(key))
        {
            existing.1 = value.clone();
        } else {
            merged.push((key.clone(), value.clone()));
        }
    }
    merged.sort_by(|a, b| a.0.cmp(&b.0));
    merged
}

/// 认证信息 → 请求头（Bearer / Basic / ApiKey，空值忽略）。
pub(crate) fn auth_headers(auth: &AuthInfo) -> Vec<(String, String)> {
    match auth {
        AuthInfo::Bearer { token } if !token.is_empty() => {
            vec![("Authorization".into(), format!("Bearer {token}"))]
        }
        AuthInfo::Basic { username, password } => {
            let encoded =
                base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}"));
            vec![("Authorization".into(), format!("Basic {encoded}"))]
        }
        AuthInfo::ApiKey { key, value } if !key.trim().is_empty() && !value.is_empty() => {
            vec![(key.clone(), value.clone())]
        }
        _ => Vec::new(),
    }
}

/// 请求头列表是否已含指定头名（大小写不敏感）。
pub(crate) fn has_header(headers: &[(String, String)], name: &str) -> bool {
    headers
        .iter()
        .any(|(key, _)| key.eq_ignore_ascii_case(name))
}
