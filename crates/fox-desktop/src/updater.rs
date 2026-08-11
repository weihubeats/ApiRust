//! 应用更新：从 GitHub Releases 检查新版本、下载更新包并打开。
//!
//! 版本来源：发布到 GitHub Release 的 tag（如 `v1.2.3`）。
//! 资源匹配：按平台选择 `macos-aarch64 / macos-x86_64 / linux-* / windows-*` 命名后缀的产物
//! （与 scripts/package.sh、scripts/package.bat 的产物命名保持一致）。

use std::path::{Path, PathBuf};

use dioxus::prelude::*;
use serde::Deserialize;

use crate::state::AppState;

/// GitHub Releases 最新版本 API（tag_name 形如 `v1.2.3`）。
const LATEST_API: &str = "https://api.github.com/repos/weihubeats/ApiRust/releases/latest";
/// 检查 / 下载超时（秒）。
const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

/// 一次更新信息（最新版本 + 发布说明 + 当前平台安装包）。
#[derive(Debug, Clone, PartialEq)]
pub struct UpdateInfo {
    /// 最新版本号（不带 v 前缀）。
    pub version: String,
    /// 发布说明（Markdown 原文）。
    pub notes: String,
    /// 安装包文件名。
    pub file_name: String,
    /// 安装包下载地址。
    pub download_url: String,
    /// 安装包大小（字节）。
    pub size: u64,
}

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
    body: Option<String>,
    assets: Vec<GhAsset>,
}

#[derive(Deserialize, Clone)]
struct GhAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(format!("RustFox/{}", env!("CARGO_PKG_VERSION")))
        .connect_timeout(REQUEST_TIMEOUT)
        .build()
        .expect("构建 HTTP 客户端失败")
}

/// 请求 GitHub 最新 Release。失败返回中文错误。
pub async fn fetch_latest_release() -> Result<UpdateInfo, String> {
    let client = http_client();
    let resp = client
        .get(LATEST_API)
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(|e| format!("网络请求失败：{e}"))?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("读取响应失败：{e}"))?;
    if !status.is_success() {
        // GitHub API 限流会返回 403 + 明确 message。
        return Err(format!(
            "检查更新失败（HTTP {}）：{}",
            status.as_u16(),
            hint(&text)
        ));
    }
    let release: GhRelease =
        serde_json::from_str(&text).map_err(|e| format!("解析更新信息失败：{e}"))?;
    let asset = pick_asset(&release.assets).ok_or_else(|| {
        format!(
            "当前平台（{}）暂无可用安装包，请前往 GitHub Releases 手动下载",
            platform_key()
        )
    })?;
    Ok(UpdateInfo {
        version: normalize_version(&release.tag_name),
        notes: release.body.unwrap_or_default(),
        file_name: asset.name,
        download_url: asset.browser_download_url,
        size: asset.size,
    })
}

/// 从响应文本提取 GitHub 限流提示（保持中文错误简洁）。
fn hint(text: &str) -> String {
    serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .and_then(|v| v.get("message").and_then(|m| m.as_str()).map(String::from))
        .unwrap_or_else(|| "请稍后重试".into())
}

/// 按当前平台选择安装包：优先精确匹配（含架构），再回退到仅匹配系统名。
fn pick_asset(assets: &[GhAsset]) -> Option<GhAsset> {
    let key = platform_key();
    let os = std::env::consts::OS;
    assets
        .iter()
        .find(|a| !key.is_empty() && a.name.contains(key))
        .or_else(|| assets.iter().find(|a| a.name.contains(os)))
        .cloned()
}

/// 当前平台产物后缀（与打包脚本命名一致）。
fn platform_key() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "macos-aarch64",
        ("macos", _) => "macos-x86_64",
        ("linux", "aarch64") => "linux-aarch64",
        ("linux", _) => "linux-x86_64",
        ("windows", _) => "windows-x86_64",
        _ => "",
    }
}

/// 去掉版本号前缀的 v（`v1.2.3` → `1.2.3`）。
fn normalize_version(v: &str) -> String {
    let t = v.trim();
    t.strip_prefix('v').unwrap_or(t).to_string()
}

/// 版本比较：a 是否严格大于 b（数字段逐段比较，缺失段视为 0；预发布版本小于正式版本）。
pub fn version_gt(a: &str, b: &str) -> bool {
    let av = parse_segments(a);
    let bv = parse_segments(b);
    for (x, y) in av.iter().zip(bv.iter()) {
        if x != y {
            return x > y;
        }
    }
    if av.len() != bv.len() {
        return av.len() > bv.len() && av[bv.len()] > 0;
    }
    // 段数相同：预发布版本（带 -alpha/-beta 等后缀）小于正式版本。
    if is_prerelease(a) != is_prerelease(b) {
        return !is_prerelease(a) && is_prerelease(b);
    }
    false
}

/// 是否预发布版本（数字/点之后仍存在其他字符，如 `1.2.3-beta`）。
fn is_prerelease(v: &str) -> bool {
    let t = normalize_version(v);
    t.chars()
        .skip_while(|c| c.is_ascii_digit() || *c == '.')
        .any(|c| c != '.')
}

/// 把版本号拆成数字段（`1.2.3-beta` → [1, 2, 3]；非数字段忽略）。
fn parse_segments(v: &str) -> Vec<u64> {
    v.split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect()
}

/// 下载更新包到 {data_dir}/updates/，期间通过回调上报进度（0.0 ~ 100.0）。
pub async fn download_update(
    url: &str,
    file_name: &str,
    mut on_progress: impl FnMut(f64),
) -> Result<PathBuf, String> {
    let dir = update_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建下载目录失败：{e}"))?;
    let path = dir.join(sanitize_file_name(file_name));

    let client = http_client();
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("下载失败：{e}"))?;
    if !resp.status().is_success() {
        return Err(format!("下载失败（HTTP {}）", resp.status().as_u16()));
    }
    let total = resp.content_length().unwrap_or(0).max(1);
    let mut stream = resp.bytes_stream();
    let mut received: u64 = 0;
    let mut out = tokio::fs::File::create(&path)
        .await
        .map_err(|e| format!("创建文件失败：{e}"))?;
    let mut last_report = std::time::Instant::now();
    use futures::StreamExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("下载中断：{e}"))?;
        received += chunk.len() as u64;
        use tokio::io::AsyncWriteExt;
        out.write_all(&chunk)
            .await
            .map_err(|e| format!("写入文件失败：{e}"))?;
        // 进度节流：每 50ms 或最终块才回调，避免 UI 渲染风暴。
        if last_report.elapsed().as_millis() >= 50 {
            on_progress(received as f64 / total as f64 * 100.0);
            last_report = std::time::Instant::now();
        }
    }
    out.sync_all()
        .await
        .map_err(|e| format!("写入文件失败：{e}"))?;
    on_progress(100.0);
    Ok(path)
}

/// 下载目录：{data_dir}/updates/。
pub fn update_dir() -> PathBuf {
    fox_storage::db::data_dir().join("updates")
}

/// 文件名校验：去掉路径分隔符，防止下载路径被污染。
fn sanitize_file_name(name: &str) -> String {
    name.replace(['/', '\\'], "_")
}

/// 用系统默认方式打开文件（macOS open / Windows explorer / Linux xdg-open）。
pub fn open_path(path: &Path) -> Result<(), String> {
    let Some(s) = path.to_str() else {
        return Err("路径不是合法文本".into());
    };
    let result = if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(s).spawn()
    } else if cfg!(target_os = "windows") {
        std::process::Command::new("explorer").arg(s).spawn()
    } else {
        std::process::Command::new("xdg-open").arg(s).spawn()
    };
    result.map(|_| ()).map_err(|e| format!("打开文件失败：{e}"))
}

/// 应用信息（设置页「关于」展示）。
pub fn about_meta() -> Vec<(String, String)> {
    let mut rows = vec![
        ("应用名称".into(), "RustFox".into()),
        ("版本".into(), format!("v{}", env!("CARGO_PKG_VERSION"))),
        (
            "构建类型".into(),
            if cfg!(debug_assertions) {
                "Debug".into()
            } else {
                "Release".into()
            },
        ),
        (
            "操作系统".into(),
            format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
        ),
        (
            "数据目录".into(),
            fox_storage::db::data_dir().display().to_string(),
        ),
        (
            "代码仓库".into(),
            "https://github.com/weihubeats/ApiRust".into(),
        ),
    ];
    if let Ok(cpu) = std::env::var("RUSTFOX_BUILD_DATE") {
        rows.push(("构建时间".into(), cpu));
    }
    rows
}

/// 更新弹窗：展示新版本信息、下载进度、下载完成后的打开入口。
#[allow(non_snake_case)]
pub fn UpdateModal() -> Element {
    let state = use_context::<AppState>();
    let open = *state.update_modal_open.read();
    if !open {
        return None;
    }
    let info = state.update_info.read().clone();
    let checking = *state.update_checking.read();
    let progress = *state.update_progress.read();
    let downloaded = state.update_downloaded.read().clone();
    let error = state.update_error.read().clone();

    let close_btn = state.clone();
    let close_btn2 = state.clone();
    let backdrop_btn = state.clone();
    let download_btn = state.clone();
    let skip_btn = state.clone();
    let open_btn = state.clone();

    rsx! {
        div {
            class: "modal-backdrop",
            onclick: move |_| {
                if progress.is_none() {
                    backdrop_btn.close_update_modal();
                }
            },
            div {
                class: "modal update-modal",
                onclick: |e| { e.stop_propagation(); },
                if let Some(info) = info {
                    h3 { "发现新版本 v{info.version}" }
                    div { class: "hint",
                        "当前版本 v{env!(\"CARGO_PKG_VERSION\")} · 更新包 {info.file_name}（{fmt_size(info.size)}）"
                    }
                    div { class: "update-notes", "{info.notes}" }
                    if let Some(p) = progress {
                        div { class: "update-progress",
                            div { class: "update-bar",
                                div { class: "update-bar-fill", style: "width: {p}%" }
                            }
                            span { class: "hint-inline", "正在下载… {p:.0}%" }
                        }
                    } else if let Some(err) = &error {
                        div { class: "update-error", "{err}" }
                    } else if downloaded.is_some() {
                        div { class: "row",
                            span { class: "mock-status ok", "下载完成" }
                            span { class: "hint-inline", "已保存到 {download_dir_display()}，请按提示安装。" }
                        }
                    }
                    div { class: "rf-modal-actions",
                        if progress.is_none() {
                            if downloaded.is_some() {
                                button {
                                    class: "rf-btn rf-btn-primary",
                                    onclick: move |_| open_btn.open_update_file(),
                                    "打开安装包",
                                }
                                button {
                                    class: "rf-btn",
                                    onclick: move |_| close_btn2.close_update_modal(),
                                    "关闭",
                                }
                            } else {
                                button {
                                    class: "rf-btn rf-btn-primary",
                                    onclick: move |_| download_btn.download_update(),
                                    "下载并更新",
                                }
                                button {
                                    class: "rf-btn",
                                    onclick: move |_| skip_btn.skip_update_version(),
                                    "跳过此版本",
                                }
                                button {
                                    class: "rf-btn",
                                    onclick: move |_| close_btn.close_update_modal(),
                                    "稍后再说",
                                }
                            }
                        } else {
                            span { class: "hint-inline", "下载中请勿关闭窗口…" }
                        }
                    }
                } else if checking {
                    div {
                        class: "row",
                        span { class: "hint-inline", "正在检查更新…" }
                    }
                } else if let Some(err) = &error {
                    h3 { "检查更新失败" }
                    div { class: "update-error", "{err}" }
                    div { class: "rf-modal-actions",
                        button {
                            class: "rf-btn",
                            onclick: move |_| close_btn2.close_update_modal(),
                            "关闭",
                        }
                    }
                }
            }
        }
    }
}

/// 下载目录展示文本（与弹窗提示共用）。
fn download_dir_display() -> String {
    update_dir().display().to_string()
}

/// 字节数人性化显示。
pub fn fmt_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_v_prefix() {
        assert_eq!(normalize_version("v1.2.3"), "1.2.3");
        assert_eq!(normalize_version("1.2.3"), "1.2.3");
        assert_eq!(normalize_version("  v1.0  "), "1.0");
    }

    #[test]
    fn version_gt_compares_segment_wise() {
        assert!(version_gt("1.2.3", "1.2.2"));
        assert!(version_gt("1.3.0", "1.2.9"));
        assert!(version_gt("2.0.0", "1.99.99"));
        assert!(version_gt("1.2.3", "1.2.3-beta"));
        assert!(!version_gt("1.2.3", "1.2.3"));
        assert!(!version_gt("1.2.3", "1.2.4"));
        assert!(!version_gt("1.2.3", "1.3.0"));
    }

    #[test]
    fn version_gt_handles_v_prefix_and_prerelease() {
        assert!(version_gt("v1.5.0", "1.4.9"));
        assert!(!version_gt("v1.2.3", "v1.2.3"));
        assert!(version_gt("1.10.0", "1.9.9"));
        assert!(version_gt("0.1.1", "0.1.0"));
        assert!(!version_gt("0.1.0", "0.1.0"));
    }

    fn asset(name: &str) -> GhAsset {
        GhAsset {
            name: name.into(),
            browser_download_url: format!("https://example.com/{name}"),
            size: 1024,
        }
    }

    #[test]
    fn pick_asset_prefers_arch_specific() {
        let assets = vec![
            asset("RustFox-1.0.0-macos-x86_64.zip"),
            asset("RustFox-1.0.0-macos-aarch64.zip"),
        ];
        // 按当前平台断言：不依赖具体平台，验证「精确匹配优先」逻辑。
        let key = platform_key();
        let picked = pick_asset(&assets).unwrap();
        if key.contains("aarch64") {
            assert!(picked.name.contains("aarch64"));
        } else {
            assert!(picked.name.contains("x86_64"));
        }
    }

    #[test]
    fn pick_asset_falls_back_to_os() {
        let os = std::env::consts::OS;
        let assets = vec![asset(&format!("RustFox-1.0.0-{os}-whatever.zip"))];
        assert!(pick_asset(&assets).is_some());
        let assets = vec![asset("RustFox-1.0.0-windows-x86_64.zip")];
        if std::env::consts::OS == "windows" {
            assert!(pick_asset(&assets).is_some());
        } else {
            assert!(
                pick_asset(&assets).is_none(),
                "不应为当前平台选中 Windows 包"
            );
        }
    }

    #[test]
    fn sanitize_removes_path_separators() {
        assert_eq!(sanitize_file_name("../evil.zip"), ".._evil.zip");
        assert_eq!(sanitize_file_name("a/b.zip"), "a_b.zip");
        assert_eq!(
            sanitize_file_name("RustFox-1.0.0-macos-aarch64.zip"),
            "RustFox-1.0.0-macos-aarch64.zip"
        );
    }

    #[test]
    fn fmt_size_human_readable() {
        assert_eq!(fmt_size(500), "500 B");
        assert_eq!(fmt_size(2048), "2.0 KB");
        assert_eq!(fmt_size(3 * 1024 * 1024), "3.0 MB");
    }
}
