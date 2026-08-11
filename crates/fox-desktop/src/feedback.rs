//! 问题反馈：收集环境信息、用户操作步骤与最近日志，生成诊断报告文件。

use std::path::PathBuf;

use chrono::Utc;
use dioxus::prelude::*;

use crate::state::AppState;

/// 报告中包含的日志行数上限。
const LOG_TAIL_LINES: usize = 500;

/// 生成反馈报告并返回文件路径。
///
/// 报告内容：① 环境信息 ② 当前上下文 ③ 最近用户操作步骤 ④ 最近日志（最多 500 行）。
pub fn generate_report(state: &AppState) -> Result<PathBuf, String> {
    let dir = fox_storage::db::data_dir().join("reports");
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建报告目录失败：{e}"))?;

    let mut md = String::new();
    md.push_str("# RustFox 问题反馈\n\n");

    md.push_str("## 环境信息\n\n");
    md.push_str(&format!(
        "- 应用版本：{}（{}，{}）\n",
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_NAME"),
        build_profile()
    ));
    md.push_str(&format!(
        "- 操作系统：{} {}（{} 架构）\n",
        std::env::consts::OS,
        os_version(),
        std::env::consts::ARCH
    ));
    md.push_str(&format!(
        "- 数据目录：{}\n",
        fox_storage::db::data_dir().display()
    ));
    md.push_str(&format!(
        "- 日志文件：{}\n",
        fox_storage::db::log_dir().join("rustfox.log").display()
    ));

    md.push_str("\n## 当前上下文\n\n");
    if let Some(pid) = *state.current_project_id.read() {
        md.push_str(&format!("- 当前项目：{pid}\n"));
    } else {
        md.push_str("- 当前项目：无\n");
    }
    if let Some(eid) = *state.current_environment_id.read() {
        md.push_str(&format!("- 当前环境：{eid}\n"));
    } else {
        md.push_str("- 当前环境：无\n");
    }
    md.push_str(&format!(
        "- Mock 服务：{}",
        if state.mock_running() {
            format!("运行中（port {}）\n", state.mock_port.read().unwrap_or(0))
        } else {
            "未运行\n".to_string()
        }
    ));

    md.push_str("\n## 最近操作步骤\n\n");
    let steps = state.steps.read();
    if steps.is_empty() {
        md.push_str("（暂无操作记录）\n");
    } else {
        for (i, s) in steps.iter().enumerate() {
            md.push_str(&format!("{}. {}\n", i + 1, s));
        }
    }

    md.push_str("\n## 日志（最近 500 行）\n\n```text\n");
    md.push_str(&read_log_tail(LOG_TAIL_LINES));
    md.push_str("```\n");

    let filename = format!("rustfox_report_{}.md", Utc::now().format("%Y%m%d_%H%M%S"));
    let path = dir.join(filename);
    std::fs::write(&path, md).map_err(|e| format!("写报告失败：{e}"))?;
    Ok(path)
}

/// 反向分块读取的块大小（8 KB）。
const TAIL_CHUNK: u64 = 8 * 1024;

/// 读取日志文件末尾若干行。
///
/// 从文件末尾向前分块读取，收集到足够换行符后立即停止，
/// 仅最后 `max_lines` 行驻留内存，避免 `read_to_string` 把
/// 数百 MB 的日志整体载入导致 UI 冻结 / OOM。
fn read_log_tail(max_lines: usize) -> String {
    let path = fox_storage::db::log_dir().join("rustfox.log");
    read_tail_from(&path, max_lines)
}

/// `read_log_tail` 的内核实现（路径注入，便于测试）。
fn read_tail_from(path: &std::path::Path, max_lines: usize) -> String {
    use std::io::{Read, Seek, SeekFrom};

    let mut file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return "（日志文件不可读）".to_string(),
    };
    let Ok(len) = file.metadata().map(|m| m.len()) else {
        return "（日志文件不可读）".to_string();
    };
    if len == 0 || max_lines == 0 {
        return String::new();
    }

    // 从文件尾向前分块读取，直到换行符数量达到 max_lines（保证尾部
    // 至少有 max_lines 行可截取）。chunks 保持读入顺序，最终反向拼接
    // 以恢复文件原始顺序。
    let mut chunks: Vec<Vec<u8>> = Vec::new();
    let mut newlines = 0usize;
    let mut cursor = len;
    while cursor > 0 && newlines < max_lines {
        let n = cursor.min(TAIL_CHUNK);
        let start = cursor - n;
        if file.seek(SeekFrom::Start(start)).is_err() {
            return "（日志文件不可读）".to_string();
        }
        let mut chunk = vec![0u8; n as usize];
        if file.read_exact(&mut chunk).is_err() {
            return "（日志文件不可读）".to_string();
        }
        newlines += chunk.iter().filter(|&&b| b == b'\n').count();
        chunks.push(chunk);
        cursor = start;
    }

    let mut tail = Vec::new();
    for chunk in chunks.into_iter().rev() {
        tail.extend_from_slice(&chunk);
    }

    let text = String::from_utf8_lossy(&tail);
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(max_lines);
    lines[start..].join("\n")
}

/// 构建类型（debug/release）。
fn build_profile() -> String {
    if cfg!(debug_assertions) {
        "debug".into()
    } else {
        "release".into()
    }
}

/// 操作系统版本（macOS 使用 sw_vers，其他平台尽力而为；失败返回空串）。
fn os_version() -> String {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_default()
    }
    #[cfg(not(target_os = "macos"))]
    {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_file(content: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "rustfox_tail_test_{}_{}.log",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn tail_returns_last_lines_of_large_file() {
        // 构造超过 8KB 块的日志，验证跨块反向读取。
        let content: String = (0..2000).map(|i| format!("line {i}\n")).collect();
        assert!(content.len() > TAIL_CHUNK as usize);
        let path = tmp_file(&content);
        let tail = read_tail_from(&path, 500);
        std::fs::remove_file(&path).ok();
        let lines: Vec<&str> = tail.lines().collect();
        assert_eq!(lines.len(), 500);
        assert_eq!(lines[0], "line 1500");
        assert_eq!(lines[499], "line 1999");
    }

    #[test]
    fn tail_without_trailing_newline_includes_last_line() {
        let path = tmp_file("a\nb\nc\nd");
        let tail = read_tail_from(&path, 2);
        std::fs::remove_file(&path).ok();
        assert_eq!(tail, "c\nd");
    }

    #[test]
    fn tail_smaller_than_request_returns_all() {
        let path = tmp_file("x\ny\n");
        let tail = read_tail_from(&path, 500);
        std::fs::remove_file(&path).ok();
        assert_eq!(tail, "x\ny");
    }

    #[test]
    fn tail_empty_file_returns_empty() {
        let path = tmp_file("");
        let tail = read_tail_from(&path, 500);
        std::fs::remove_file(&path).ok();
        assert_eq!(tail, "");
    }

    #[test]
    fn tail_missing_file_returns_unreadable() {
        let path = PathBuf::from("/nonexistent/rustfox_missing.log");
        assert_eq!(read_tail_from(&path, 500), "（日志文件不可读）");
    }
}
