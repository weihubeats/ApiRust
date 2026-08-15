//! 全局状态管理器：SQLite 连接 + 当前激活 Project / Environment 缓存。
//!
//! - 使用 `tokio::sync::RwLock`（读多写少），Command 并发读取激活上下文；
//! - 激活对象首次访问时从数据库加载并写回缓存，避免重复查询；
//! - `variables_for` 提供「环境 > 项目」合并变量表，供请求渲染使用。

use std::collections::HashMap;

use fox_core::model::{Environment, Project};
use fox_core::VariableMap;
use sqlx::SqlitePool;
use tokio::sync::RwLock;
use uuid::Uuid;

use fox_storage::repository as repo;

use crate::error::CommandResult;

/// 当前激活上下文（多标签 / 多窗口共享）。
#[derive(Debug, Default)]
pub struct ActiveContext {
    pub project_id: Option<Uuid>,
    /// 缓存的项目（避免重复查询）。
    pub project: Option<Project>,
    pub environment_id: Option<Uuid>,
    /// 缓存的环境。
    pub environment: Option<Environment>,
}

/// 应用全局状态，由插件在 `setup` 中 `app.manage()` 托管。
pub struct AppState {
    pub db: SqlitePool,
    /// 激活上下文（读写并发安全）。
    pub active: RwLock<ActiveContext>,
    /// 正在运行的 Mock 服务（未启动为 `None`）。
    pub mock: RwLock<Option<fox_mock::server::MockServer>>,
}

impl AppState {
    pub fn new(db: SqlitePool) -> Self {
        AppState {
            db,
            active: RwLock::new(ActiveContext::default()),
            mock: RwLock::new(None),
        }
    }

    /// 当前激活项目（缓存命中直接返回；否则查询并写回缓存）。
    pub async fn active_project(&self) -> CommandResult<Option<Project>> {
        let read = self.active.read().await;
        if let Some(project) = &read.project {
            return Ok(Some(project.clone()));
        }
        let Some(id) = read.project_id else {
            return Ok(None);
        };
        drop(read);
        let project = repo::get_project(&self.db, id).await?;
        let mut write = self.active.write().await;
        write.project = Some(project.clone());
        Ok(Some(project))
    }

    /// 当前激活环境（缓存命中直接返回；否则查询并写回缓存）。
    pub async fn active_environment(&self) -> CommandResult<Option<Environment>> {
        let read = self.active.read().await;
        if let Some(environment) = &read.environment {
            return Ok(Some(environment.clone()));
        }
        let Some(id) = read.environment_id else {
            return Ok(None);
        };
        drop(read);
        let environment = repo::get_environment(&self.db, id).await?;
        let mut write = self.active.write().await;
        write.environment = Some(environment.clone());
        Ok(Some(environment))
    }

    /// 设置激活项目（`None` 表示清空）。项目切换后，属于其他项目的环境自动失效。
    pub async fn set_active_project(&self, project_id: Option<Uuid>) -> CommandResult<()> {
        let mut write = self.active.write().await;
        write.project_id = project_id;
        write.project = match project_id {
            Some(id) => Some(repo::get_project(&self.db, id).await?),
            None => None,
        };
        let env_belongs = write
            .environment
            .as_ref()
            .map(|env| Some(env.project_id))
            .unwrap_or(None);
        if env_belongs != project_id {
            write.environment_id = None;
            write.environment = None;
        }
        Ok(())
    }

    /// 设置激活环境（`None` 表示不使用环境变量）。
    pub async fn set_active_environment(&self, environment_id: Option<Uuid>) -> CommandResult<()> {
        let mut write = self.active.write().await;
        write.environment_id = environment_id;
        write.environment = match environment_id {
            Some(id) => Some(repo::get_environment(&self.db, id).await?),
            None => None,
        };
        Ok(())
    }

    /// 合并变量表：运行时（空）> 环境 > 项目。
    pub async fn variables_for(&self, environment_id: Option<Uuid>) -> CommandResult<VariableMap> {
        let project = self.active_project().await?;
        let environment = match environment_id {
            // 单次请求显式指定环境：临时加载，不改动全局激活状态。
            Some(id) => Some(repo::get_environment(&self.db, id).await?),
            None => self.active_environment().await?,
        };
        let project_vars = project.map(|p| p.variables).unwrap_or_default();
        let environment_vars = environment.map(|e| e.variables).unwrap_or_default();
        Ok(fox_core::merge_variables(
            &HashMap::new(),
            &environment_vars,
            &project_vars,
        ))
    }
}
