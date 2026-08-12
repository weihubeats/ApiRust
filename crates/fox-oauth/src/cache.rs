//! OAuth2 token 进程内缓存。
//!
//! key = (client_id, token_url)。读是同步的（`cached_token`），
//! 供 UI（dioxus）与请求渲染同步取用；刷新由 [`crate::client`] 在
//! 每 key 的 async 锁内串行执行。

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use fox_core::model::OAuth2Token;

type Key = (String, String);

static CACHE: OnceLock<Mutex<HashMap<Key, OAuth2Token>>> = OnceLock::new();

fn cache() -> &'static Mutex<HashMap<Key, OAuth2Token>> {
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 读取缓存中的令牌（可能已过期，由调用方判断）。
pub fn cached_token(client_id: &str, token_url: &str) -> Option<OAuth2Token> {
    cache().lock().ok().and_then(|m| {
        m.get(&(client_id.to_string(), token_url.to_string()))
            .cloned()
    })
}

/// 写入缓存。
pub fn store_token(client_id: &str, token_url: &str, token: OAuth2Token) {
    if let Ok(mut m) = cache().lock() {
        m.insert((client_id.to_string(), token_url.to_string()), token);
    }
}

/// 清除缓存中的令牌（取消授权时调用；不修改持久化的 AuthSpec）。
pub fn clear_token(client_id: &str, token_url: &str) {
    if let Ok(mut m) = cache().lock() {
        m.remove(&(client_id.to_string(), token_url.to_string()));
    }
}

/// 清空全部缓存（测试用）。
pub fn reset_cache() {
    if let Ok(mut m) = cache().lock() {
        m.clear();
    }
}

/// 测试串行锁：所有触碰全局缓存的测试共享，避免并行执行互相污染。
#[cfg(test)]
pub(crate) fn test_lock() -> &'static std::sync::Mutex<()> {
    static L: std::sync::Mutex<()> = std::sync::Mutex::new(());
    &L
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    fn token() -> OAuth2Token {
        OAuth2Token {
            access_token: "at".into(),
            token_type: "Bearer".into(),
            refresh_token: Some("rt".into()),
            expires_at: Utc::now() + Duration::hours(1),
        }
    }

    #[test]
    fn store_read_clear_roundtrip() {
        let _guard = test_lock().lock().unwrap();
        reset_cache();
        assert!(cached_token("c1", "https://t/x").is_none());
        store_token("c1", "https://t/x", token());
        assert_eq!(
            cached_token("c1", "https://t/x").unwrap().access_token,
            "at"
        );
        // key 不同 → 相互隔离
        assert!(cached_token("c1", "https://t/y").is_none());
        clear_token("c1", "https://t/x");
        assert!(cached_token("c1", "https://t/x").is_none());
    }

    #[test]
    fn store_overwrites_previous() {
        let _guard = test_lock().lock().unwrap();
        reset_cache();
        let mut t = token();
        store_token("k", "u", t.clone());
        t.access_token = "at-2".into();
        store_token("k", "u", t);
        assert_eq!(cached_token("k", "u").unwrap().access_token, "at-2");
    }
}
