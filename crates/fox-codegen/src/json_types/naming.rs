//! 字段/类型名风格转换与关键字防护。
//!
//! 输入 JSON 的 key 可能是 snake_case / camelCase / PascalCase / kebab-case
//! 乃至混合风格。统一先拆词（tokenize），再按目标语言规范组合：
//! - Rust 强制 snake_case；Java 字段 camelCase；Go 字段 PascalCase（导出）；
//!   结构体名一律 PascalCase。
//! - Rust 关键字加 `r#` 前缀；Java 关键字加 `_` 前缀；数字开头加保护前缀。

/// 将任意风格标识符拆成单词列表。
///
/// 边界规则：
/// - 分隔符：`_` `-` `.` 空格 `/` `\` `:`
/// - camel 边界：小写→大写（`userName` → `user`, `Name`）
/// - 全大写缩写后接小写（`URLValue` → `URL`, `Value`）
/// - 字母↔数字转换（`api2x` → `api`, `2x`）
fn split_words(input: &str) -> Vec<String> {
    let chars: Vec<char> = input.chars().collect();
    let mut words: Vec<String> = Vec::new();
    let mut current = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if matches!(c, '_' | '-' | '.' | ' ' | '/' | '\\' | ':') {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
            continue;
        }
        let prev = chars.get(i.wrapping_sub(1)).copied();
        let next = chars.get(i + 1).copied();
        let boundary = match prev {
            Some(p) if p.is_lowercase() && c.is_uppercase() => true, // camelCase
            Some(p)
                if p.is_uppercase()
                    && c.is_uppercase()
                    && next.is_some_and(|n| n.is_lowercase()) =>
            {
                true // URLValue
            }
            Some(p) if p.is_alphabetic() && c.is_numeric() => true, // api2x → api, 2x
            _ => false,
        };
        if boundary && !current.is_empty() {
            words.push(std::mem::take(&mut current));
        }
        current.push(c);
    }
    if !current.is_empty() {
        words.push(current);
    }
    if words.is_empty() {
        words.push(input.to_string());
    }
    words
}

/// 首字母大写（`id` → `Id`，`ID` → `ID`，保留内部大小写）。
fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Rust 约定：全小写下划线连接。
pub(crate) fn to_snake_case(input: &str) -> String {
    split_words(input)
        .iter()
        .map(|word| word.to_lowercase())
        .collect::<Vec<_>>()
        .join("_")
}

/// Java/JS 约定：首个词小写，后续词首字母大写。
pub(crate) fn to_camel_case(input: &str) -> String {
    let mut out = String::new();
    for (i, word) in split_words(input).iter().enumerate() {
        if i == 0 {
            out.push_str(&word.to_lowercase());
        } else {
            out.push_str(&capitalize(word));
        }
    }
    out
}

/// 结构体/类约定：每个词首字母大写。
pub(crate) fn to_pascal_case(input: &str) -> String {
    split_words(input).iter().map(|w| capitalize(w)).collect()
}

/// 标识符不能以数字开头时加保护前缀（Rust/Java 用 `_`，Go 用 `N` 保持导出）。
fn guard_digit(ident: String, prefix: &str) -> String {
    match ident.chars().next() {
        Some(first) if first.is_ascii_digit() => format!("{prefix}{ident}"),
        None => format!("{prefix}Empty"),
        _ => ident,
    }
}

/// Rust 关键字段名：关键字加 `r#`（2018+ raw identifier）。
pub(crate) fn rust_field_ident(snake: String) -> String {
    const KEYWORDS: &[&str] = &[
        "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
        "extern", "false", "fn", "for", "gen", "if", "impl", "in", "let", "loop", "match", "mod",
        "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super",
        "trait", "true", "try", "type", "unsafe", "use", "where", "while",
    ];
    let guarded = guard_digit(snake, "_");
    if KEYWORDS.contains(&guarded.as_str()) {
        format!("r#{guarded}")
    } else {
        guarded
    }
}

/// Java 关键字段名：加 `_` 前缀。
pub(crate) fn java_field_ident(camel: String) -> String {
    const KEYWORDS: &[&str] = &[
        "abstract",
        "assert",
        "boolean",
        "break",
        "byte",
        "case",
        "catch",
        "char",
        "class",
        "const",
        "continue",
        "default",
        "do",
        "double",
        "else",
        "enum",
        "extends",
        "final",
        "finally",
        "float",
        "for",
        "goto",
        "if",
        "implements",
        "import",
        "instanceof",
        "int",
        "interface",
        "long",
        "native",
        "new",
        "null",
        "package",
        "permits",
        "private",
        "protected",
        "public",
        "record",
        "return",
        "sealed",
        "short",
        "static",
        "strictfp",
        "super",
        "switch",
        "synchronized",
        "this",
        "throw",
        "throws",
        "transient",
        "true",
        "try",
        "var",
        "void",
        "volatile",
        "while",
        "yield",
    ];
    let guarded = guard_digit(camel, "_");
    if KEYWORDS.contains(&guarded.as_str()) {
        format!("_{guarded}")
    } else {
        guarded
    }
}

/// Go 字段名：PascalCase（导出），数字开头加 `N` 前缀。
pub(crate) fn go_field_ident(pascal: String) -> String {
    guard_digit(pascal, "N")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snake_case_variants() {
        assert_eq!(to_snake_case("userName"), "user_name");
        assert_eq!(to_snake_case("user_name"), "user_name");
        assert_eq!(to_snake_case("user-name"), "user_name");
        assert_eq!(to_snake_case("UserID"), "user_id");
        assert_eq!(to_snake_case("URLValue"), "url_value");
        assert_eq!(to_snake_case("api2x"), "api_2x");
        assert_eq!(to_snake_case("user.name"), "user_name");
    }

    #[test]
    fn camel_case_variants() {
        assert_eq!(to_camel_case("user_id"), "userId");
        assert_eq!(to_camel_case("User-Name"), "userName");
        assert_eq!(to_camel_case("phoneNumber"), "phoneNumber");
    }

    #[test]
    fn pascal_case_variants() {
        assert_eq!(to_pascal_case("user_id"), "UserId");
        assert_eq!(to_pascal_case("userName"), "UserName");
        assert_eq!(to_pascal_case("__"), "__");
    }

    #[test]
    fn keyword_guards() {
        assert_eq!(rust_field_ident("type".into()), "r#type");
        assert_eq!(rust_field_ident("loop".into()), "r#loop");
        assert_eq!(rust_field_ident("name".into()), "name");
        assert_eq!(java_field_ident("class".into()), "_class");
        assert_eq!(java_field_ident("name".into()), "name");
    }

    #[test]
    fn digit_leading_guards() {
        assert_eq!(rust_field_ident("2fa".into()), "_2fa");
        assert_eq!(java_field_ident("2fa".into()), "_2fa");
        assert_eq!(go_field_ident("2Fa".into()), "N2Fa");
    }
}
