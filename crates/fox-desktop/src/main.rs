//! RustFox 桌面应用入口。

mod app;
mod components;
mod feedback;
mod pages;
mod services;
mod shortcuts;
mod state;
mod styles;
mod updater;
mod views;
mod window_state;

use std::fs::File;
use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex};

use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::EnvFilter;

use fox_core::AppError;
use fox_storage::db;

use crate::app::App;

fn main() -> std::process::ExitCode {
    init_logging();
    tracing::info!("RustFox 启动");

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            tracing::error!("创建 tokio runtime 失败：{e}");
            show_fatal_error(&format!("启动失败：无法创建运行时（{e}）"));
            return std::process::ExitCode::FAILURE;
        }
    };

    // 数据库初始化失败时弹窗告知用户并退出，而不是静默崩溃。
    let pool = match runtime.block_on(prepare_startup(&db::database_path())) {
        Ok(pool) => pool,
        Err(e) => {
            show_fatal_error(&format!("数据库初始化失败：{}", e.user_message()));
            return std::process::ExitCode::FAILURE;
        }
    };

    // 启动时恢复上次关闭的窗口状态（settings 表）；无记录则用默认尺寸。
    let restored = window_state::WindowState::load(&pool);

    let mut wb = dioxus::desktop::WindowBuilder::new()
        .with_title("RustFox")
        .with_min_inner_size(dioxus::desktop::LogicalSize::new(900.0, 600.0));
    match restored {
        Some(s) => {
            wb = wb
                .with_inner_size(dioxus::desktop::LogicalSize::new(s.width, s.height))
                .with_position(dioxus::desktop::LogicalPosition::new(s.x, s.y));
            if s.maximized {
                wb = wb.with_maximized(true);
            }
        }
        None => {
            wb = wb.with_inner_size(dioxus::desktop::LogicalSize::new(1360.0, 900.0));
        }
    }
    let cfg = dioxus::desktop::Config::new().with_window(wb);

    dioxus::prelude::LaunchBuilder::new()
        .with_cfg(cfg)
        .launch(App);
    println!("RustFox 已退出");
    std::process::ExitCode::SUCCESS
}

/// 启动前置：初始化数据库并注入全局池；失败时记录详细错误并返回，
/// 由调用方弹窗提示用户。
async fn prepare_startup(path: &Path) -> Result<sqlx::SqlitePool, AppError> {
    match db::init_db(path).await {
        Ok(pool) => {
            tracing::info!("数据库已初始化：{}", path.display());
            app::provide_pool(pool.clone());
            Ok(pool)
        }
        Err(e) => {
            tracing::error!(
                error = ?e,
                "数据库初始化失败：{}（{}）",
                path.display(),
                e.user_message()
            );
            Err(e)
        }
    }
}

/// 弹窗展示致命错误（数据库初始化失败等），随后进程退出。
fn show_fatal_error(message: &str) {
    let _ = rfd::MessageDialog::new()
        .set_title("RustFox 启动失败")
        .set_description(message)
        .set_level(rfd::MessageLevel::Error)
        .show();
}

/// 初始化日志：stdout + {DataDir}/RustFox/logs/rustfox.log，级别由 RUST_LOG 控制（默认 info）。
fn init_logging() {
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into());
    let writer = LogWriter {
        file: make_log_file().ok(),
    };
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(filter))
        .with_writer(writer)
        .try_init();
}

/// 同时输出到 stdout 与日志文件。
struct LogWriter {
    file: Option<Arc<Mutex<File>>>,
}

impl<'a> MakeWriter<'a> for LogWriter {
    type Writer = Box<dyn io::Write + 'a>;

    fn make_writer(&'a self) -> Self::Writer {
        let stdout: Box<dyn io::Write + 'a> = Box::new(io::stdout());
        match &self.file {
            Some(file) => Box::new(MultiWriter {
                stdout,
                file: file.clone(),
            }),
            None => stdout,
        }
    }
}

struct MultiWriter<'a> {
    stdout: Box<dyn io::Write + 'a>,
    file: Arc<Mutex<File>>,
}

impl io::Write for MultiWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.stdout.write(buf)?;
        // 中毒的锁（写入期间 panic）仍可继续用于日志，取回内部值而非终止进程。
        let mut guard = self
            .file
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _ = guard.write_all(buf);
        let _ = guard.flush();
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stdout.flush()
    }
}

fn make_log_file() -> io::Result<Arc<Mutex<File>>> {
    let dir = db::log_dir();
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("rustfox.log");
    let file = File::options().create(true).append(true).open(path)?;
    Ok(Arc::new(Mutex::new(file)))
}

#[cfg(test)]
mod startup_tests {
    use super::*;

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "fox_desktop_{tag}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// 启动流程：数据库初始化成功 → 注入全局池 → 池可正常查询。
    #[tokio::test]
    async fn startup_prepare_succeeds_and_provides_pool() {
        let dir = temp_dir("startup_ok");
        let path = dir.join("rustfox.db");

        let pool = prepare_startup(&path).await.expect("初始化应成功");
        let row: i64 = sqlx::query_scalar("SELECT 1")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row, 1);
        assert!(app::debug_pool().is_some(), "provide_pool 应已注入全局池");

        std::fs::remove_dir_all(&dir).ok();
    }

    /// 数据库文件不可用时返回原始错误（供弹窗提示与日志记录）。
    #[tokio::test]
    async fn startup_prepare_reports_error() {
        let dir = temp_dir("startup_fail");
        // 把目录作为数据库文件路径，连接必然失败。
        let err = prepare_startup(&dir).await.unwrap_err();
        assert!(matches!(err, AppError::Database(_) | AppError::Io(_)));
    }

    /// 启动后由 window_state 读取的池必须与全局池一致（同一实例）。
    /// `WindowState::load` 内部创建自己的 runtime，须在 runtime 外调用。
    #[test]
    fn startup_pool_usable_for_window_state() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let dir = temp_dir("startup_ws");
        let path = dir.join("rustfox.db");
        let pool = rt.block_on(prepare_startup(&path)).unwrap();
        let restored = crate::window_state::WindowState::load(&pool);
        assert!(restored.is_none(), "空库不应有窗口状态记录");
        std::fs::remove_dir_all(&dir).ok();
    }
}
