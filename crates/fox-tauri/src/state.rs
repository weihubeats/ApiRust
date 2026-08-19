//! 全局状态管理器：SQLite 连接 + 当前激活 Project / Environment 缓存。
//!
//! - 使用 `tokio::sync::RwLock`（读多写少），Command 并发读取激活上下文；
//! - 激活对象首次访问时从数据库加载并写回缓存，避免重复查询；
//! - `variables_for` 提供「环境 > 项目」合并变量表，供请求渲染使用。

use std::collections::HashMap;
use std::sync::Mutex;

use fox_core::model::{Environment, Project};
use fox_core::VariableMap;
use sqlx::SqlitePool;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use fox_storage::repository as repo;

use crate::error::CommandResult;

/// 激活上下文持久化键（settings 表）。
const KEY_ACTIVE_PROJECT: &str = "active_project_id";
const KEY_ACTIVE_ENVIRONMENT: &str = "active_environment_id";

/// 序列化激活 id 为 settings 值（JSON：`"uuid"` 或 `null`）。
fn setting_value(id: Option<Uuid>) -> String {
    match id {
        Some(id) => serde_json::to_string(&id.to_string()).unwrap_or_else(|_| "null".into()),
        None => "null".into(),
    }
}

/// 读取持久化的激活 id（缺失 / 损坏返回 `None`）。
async fn load_setting_uuid(db: &SqlitePool, key: &str) -> Option<Uuid> {
    repo::get_setting(db, key)
        .await
        .ok()
        .flatten()
        .and_then(|v| serde_json::from_str::<Option<String>>(&v).ok().flatten())
        .and_then(|s| Uuid::parse_str(&s).ok())
}

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
    /// 在途请求的取消令牌注册表（request_id → token；「取消请求」时触发中止）。
    /// 持有期间不 await，普通 `Mutex` 即可。
    pub request_cancels: Mutex<HashMap<String, CancellationToken>>,
}

impl AppState {
    pub fn new(db: SqlitePool) -> Self {
        AppState {
            db,
            active: RwLock::new(ActiveContext::default()),
            mock: RwLock::new(None),
            request_cancels: Mutex::new(HashMap::new()),
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
    /// 持久化到 settings 表，重启后由 `restore_active` 恢复。
    pub async fn set_active_project(&self, project_id: Option<Uuid>) -> CommandResult<()> {
        let mut write = self.active.write().await;
        write.project_id = project_id;
        write.project = match project_id {
            Some(id) => Some(repo::get_project(&self.db, id).await?),
            None => None,
        };
        // 环境归属：优先用缓存，缓存缺失（如恢复后的初始状态）时查库补全。
        let env_project = match (write.environment.as_ref(), write.environment_id) {
            (Some(env), _) => Some(env.project_id),
            (None, Some(id)) => match repo::get_environment(&self.db, id).await {
                Ok(env) => Some(env.project_id),
                Err(_) => None,
            },
            (None, None) => None,
        };
        let mut env_id = write.environment_id;
        if env_project != project_id {
            write.environment_id = None;
            write.environment = None;
            env_id = None;
        }
        drop(write);
        repo::set_setting(&self.db, KEY_ACTIVE_PROJECT, &setting_value(project_id)).await?;
        repo::set_setting(&self.db, KEY_ACTIVE_ENVIRONMENT, &setting_value(env_id)).await?;
        Ok(())
    }

    /// 设置激活环境（`None` 表示不使用环境变量）。持久化到 settings 表。
    pub async fn set_active_environment(&self, environment_id: Option<Uuid>) -> CommandResult<()> {
        let mut write = self.active.write().await;
        write.environment_id = environment_id;
        write.environment = match environment_id {
            Some(id) => Some(repo::get_environment(&self.db, id).await?),
            None => None,
        };
        drop(write);
        repo::set_setting(
            &self.db,
            KEY_ACTIVE_ENVIRONMENT,
            &setting_value(environment_id),
        )
        .await?;
        Ok(())
    }

    /// 启动时恢复持久化的激活项目 / 环境（校验存在性与归属，无效则丢弃）。
    pub async fn restore_active(&self) -> CommandResult<()> {
        let project_id = load_setting_uuid(&self.db, KEY_ACTIVE_PROJECT).await;
        let project_ok = match project_id {
            Some(id) => repo::get_project(&self.db, id).await.is_ok(),
            None => false,
        };
        let environment_id = load_setting_uuid(&self.db, KEY_ACTIVE_ENVIRONMENT).await;
        let env_ok = match (project_id, environment_id) {
            (Some(pid), Some(eid)) => match repo::get_environment(&self.db, eid).await {
                Ok(env) => env.project_id == pid,
                Err(_) => false,
            },
            _ => false,
        };
        let mut write = self.active.write().await;
        write.project_id = project_id.filter(|_| project_ok);
        write.environment_id = environment_id.filter(|_| env_ok);
        write.project = None;
        write.environment = None;
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

#[cfg(test)]
mod tests {
    use super::*;
    use fox_core::model::Project;
    use std::path::PathBuf;

    /// 激活项目 / 环境必须跨「重启」恢复：写入 settings 表，重建状态后可读回。
    #[tokio::test]
    async fn active_context_persists_across_restart() {
        let path: PathBuf =
            std::env::temp_dir().join(format!("rustfox-active-test-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let db = fox_storage::db::init_db(&path).await.expect("建库");
        let state = AppState::new(db.clone());

        let project = Project {
            id: Uuid::new_v4(),
            name: "测试项目".into(),
            description: String::new(),
            variables: Default::default(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        repo::save_project(&db, &project).await.expect("落库项目");
        let env_id = Uuid::new_v4();
        repo::save_environment(
            &db,
            &Environment {
                id: env_id,
                project_id: project.id,
                name: "dev".into(),
                variables: Default::default(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            },
        )
        .await
        .expect("落库环境");

        state
            .set_active_project(Some(project.id))
            .await
            .expect("激活项目");
        state
            .set_active_environment(Some(env_id))
            .await
            .expect("激活环境");

        // 模拟重启：同库新建状态，仅靠 settings 表恢复。
        let restarted = AppState::new(db.clone());
        restarted.restore_active().await.expect("恢复激活上下文");
        let read = restarted.active.read().await;
        assert_eq!(read.project_id, Some(project.id), "项目应恢复");
        assert_eq!(read.environment_id, Some(env_id), "环境应恢复");
        drop(read);
        assert_eq!(
            restarted
                .active_environment()
                .await
                .expect("读环境")
                .map(|e| e.id),
            Some(env_id)
        );

        // 回归：重启后用户经项目列表「重新进入」同一项目，环境必须保留
        //（restore 后 environment 缓存为空，旧逻辑会误判归属并清空环境）。
        restarted
            .set_active_project(Some(project.id))
            .await
            .expect("重进项目");
        {
            let read = restarted.active.read().await;
            assert_eq!(
                read.environment_id,
                Some(env_id),
                "重进同一项目不应清空环境"
            );
        }

        // 环境不属于当前项目时应被丢弃。
        let other_project_id = Uuid::new_v4();
        repo::save_project(
            &db,
            &Project {
                id: other_project_id,
                name: "其他项目".into(),
                description: String::new(),
                variables: Default::default(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            },
        )
        .await
        .expect("落库其他项目");
        let other_env_id = Uuid::new_v4();
        repo::save_environment(
            &db,
            &Environment {
                id: other_env_id,
                project_id: other_project_id,
                name: "other".into(),
                variables: Default::default(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            },
        )
        .await
        .expect("落库其他环境");
        repo::set_setting(&db, KEY_ACTIVE_PROJECT, &setting_value(Some(project.id)))
            .await
            .expect("写项目");
        repo::set_setting(
            &db,
            KEY_ACTIVE_ENVIRONMENT,
            &setting_value(Some(other_env_id)),
        )
        .await
        .expect("写环境");
        let again = AppState::new(db.clone());
        again.restore_active().await.expect("恢复");
        let read = again.active.read().await;
        assert_eq!(read.project_id, Some(project.id));
        assert_eq!(read.environment_id, None, "跨项目环境应被丢弃");
        drop(read);

        // 切换到其他项目 → 环境应清空并持久化 null。
        restarted
            .set_active_project(Some(other_project_id))
            .await
            .expect("切换项目");
        {
            let read = restarted.active.read().await;
            assert_eq!(read.environment_id, None, "跨项目环境应清空");
        }

        db.close().await;
        let _ = std::fs::remove_file(&path);
    }
}
