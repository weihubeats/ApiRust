//! RustFox 桌面应用入口。

mod app;
mod components;
mod pages;
mod services;
mod state;
mod styles;

use std::fs::File;
use std::io;
use std::sync::{Arc, Mutex};

use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::EnvFilter;

use fox_storage::db;

use crate::app::App;

fn main() -> std::process::ExitCode {
    init_logging();
    tracing::info!("RustFox 启动");

    let pool = tokio::runtime::Runtime::new()
        .expect("创建 tokio runtime 失败")
        .block_on(async {
            match db::init_db(&db::database_path()).await {
                Ok(pool) => {
                    tracing::info!("数据库已初始化：{}", db::database_path().display());
                    Ok(pool)
                }
                Err(e) => {
                    eprintln!("数据库初始化失败：{}", e.user_message());
                    Err(())
                }
            }
        });
    let Ok(pool) = pool else {
        return std::process::ExitCode::FAILURE;
    };
    app::provide_pool(pool);

    let cfg = dioxus::desktop::Config::new().with_window(
        dioxus::desktop::WindowBuilder::new()
            .with_title("RustFox")
            .with_inner_size(dioxus::desktop::LogicalSize::new(1360.0, 900.0))
            .with_min_inner_size(dioxus::desktop::LogicalSize::new(900.0, 600.0)),
    );

    dioxus::prelude::LaunchBuilder::new()
        .with_cfg(cfg)
        .launch(App);
    println!("RustFox 已退出");
    std::process::ExitCode::SUCCESS
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
        let mut guard = self.file.lock().unwrap();
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
