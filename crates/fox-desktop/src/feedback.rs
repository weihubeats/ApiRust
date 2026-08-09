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

/// 读取日志文件末尾若干行。
fn read_log_tail(max_lines: usize) -> String {
    let path = fox_storage::db::log_dir().join("rustfox.log");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return "（日志文件不可读）".to_string();
    };
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
