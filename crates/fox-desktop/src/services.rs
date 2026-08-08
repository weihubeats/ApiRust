//! 服务句柄与项目服务封装。

use sqlx::SqlitePool;

/// 应用服务集合。
#[derive(Clone)]
pub struct Services {
    pub db: SqlitePool,
}

impl Services {
    pub fn new(db: SqlitePool) -> Self {
        Services { db }
    }
}
