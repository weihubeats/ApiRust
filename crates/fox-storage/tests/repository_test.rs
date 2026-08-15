//! fox-storage 集成测试：Project / Folder / Endpoint / Environment CRUD。

use sqlx::SqlitePool;

use std::collections::HashMap;

use fox_storage::db::memory_pool;
use fox_storage::repository as repo;

use fox_core::model::WsMessageType;

async fn pool() -> SqlitePool {
    memory_pool().await.unwrap()
}

#[tokio::test]
async fn project_crud() {
    let db = pool().await;

    let created = repo::create_project(&db, "Demo API", "描述").await.unwrap();
    assert_eq!(created.name, "Demo API");
    assert!(!created.id.to_string().is_empty());

    let listed = repo::list_projects(&db).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, created.id);
    assert_eq!(listed[0].description, "描述");

    let fetched = repo::get_project(&db, created.id).await.unwrap();
    assert_eq!(fetched.id, created.id);
    assert!(created.variables.is_empty());

    let mut updated = fetched.clone();
    updated.name = "改名".into();
    updated
        .variables
        .insert("base_url".into(), "https://x.com".into());
    repo::update_project(&db, &updated).await.unwrap();
    let refetched = repo::get_project(&db, created.id).await.unwrap();
    assert_eq!(refetched.name, "改名");
    assert_eq!(refetched.variables["base_url"], "https://x.com");

    repo::delete_project(&db, created.id).await.unwrap();
    assert!(repo::list_projects(&db).await.unwrap().is_empty());
    assert!(repo::get_project(&db, created.id).await.is_err());
}

#[tokio::test]
async fn folder_crud() {
    let db = pool().await;
    let project = repo::create_project(&db, "P", "").await.unwrap();

    let root = repo::create_folder(&db, project.id, None, "根目录")
        .await
        .unwrap();
    assert_eq!(root.name, "根目录");
    assert!(root.parent_id.is_none());

    let child = repo::create_folder(&db, project.id, Some(root.id), "子目录")
        .await
        .unwrap();
    assert_eq!(child.parent_id, Some(root.id));

    let listed = repo::list_folders(&db, project.id).await.unwrap();
    assert_eq!(listed.len(), 2);

    let fetched = repo::get_folder(&db, child.id).await.unwrap();
    assert_eq!(fetched.name, "子目录");

    // 重命名文件夹。
    let mut renamed = fetched.clone();
    renamed.name = "改名字目录".into();
    let updated = repo::update_folder(&db, &renamed).await.unwrap();
    assert_eq!(updated.name, "改名字目录");
    let fetched = repo::get_folder(&db, child.id).await.unwrap();
    assert_eq!(fetched.name, "改名字目录");

    repo::delete_folder(&db, root.id).await.unwrap();
    // 删除父文件夹后，子文件夹（及整个子树）应一并级联删除。
    assert!(repo::get_folder(&db, child.id).await.is_err());
}

#[tokio::test]
async fn delete_folder_cascades_subtree() {
    let db = pool().await;
    let project = repo::create_project(&db, "P", "").await.unwrap();

    let root = repo::create_folder(&db, project.id, None, "根")
        .await
        .unwrap();
    let child = repo::create_folder(&db, project.id, Some(root.id), "子")
        .await
        .unwrap();
    let grand = repo::create_folder(&db, project.id, Some(child.id), "孙")
        .await
        .unwrap();

    let ep_root = repo::create_endpoint(&db, project.id, Some(root.id), "R")
        .await
        .unwrap();
    let ep_child = repo::create_endpoint(&db, project.id, Some(child.id), "C")
        .await
        .unwrap();
    let ep_grand = repo::create_endpoint(&db, project.id, Some(grand.id), "G")
        .await
        .unwrap();
    let ep_free = repo::create_endpoint(&db, project.id, None, "F")
        .await
        .unwrap();

    repo::delete_folder(&db, root.id).await.unwrap();

    // 子孙文件夹全部删除，不再有孤儿记录。
    assert!(repo::get_folder(&db, root.id).await.is_err());
    assert!(repo::get_folder(&db, child.id).await.is_err());
    assert!(repo::get_folder(&db, grand.id).await.is_err());
    // 子树下接口全部删除。
    assert!(repo::get_endpoint(&db, ep_root.id).await.is_err());
    assert!(repo::get_endpoint(&db, ep_child.id).await.is_err());
    assert!(repo::get_endpoint(&db, ep_grand.id).await.is_err());
    // 子树外接口不受影响。
    assert!(repo::get_endpoint(&db, ep_free.id).await.is_ok());
    // 删除不存在的文件夹返回 NotFound。
    assert!(repo::delete_folder(&db, uuid::Uuid::new_v4())
        .await
        .is_err());
}

#[tokio::test]
async fn endpoint_crud() {
    let db = pool().await;
    let project = repo::create_project(&db, "P", "").await.unwrap();
    let folder = repo::create_folder(&db, project.id, None, "F")
        .await
        .unwrap();

    let created = repo::create_endpoint(&db, project.id, Some(folder.id), "查询用户")
        .await
        .unwrap();
    assert_eq!(created.folder_id, Some(folder.id));
    assert_eq!(created.method.to_string(), "GET");
    assert_eq!(created.status.as_str(), "developing");

    let fetched = repo::get_endpoint(&db, created.id).await.unwrap();
    assert_eq!(fetched.name, "查询用户");
    assert_eq!(fetched.request.params.len(), 0);

    let mut updated = fetched.clone();
    updated.method = "POST".parse().unwrap();
    updated.path = "/users".into();
    updated
        .request
        .params
        .push(fox_core::model::KeyValue::new("page", "1"));
    updated.request.body = fox_core::model::BodySpec::Json {
        raw: "{\"a\":1}".into(),
    };
    let saved = repo::update_endpoint(&db, &updated).await.unwrap();
    assert_eq!(saved.method.to_string(), "POST");

    let refetched = repo::get_endpoint(&db, created.id).await.unwrap();
    assert_eq!(refetched.path, "/users");
    assert_eq!(refetched.request.params[0].key, "page");
    assert_eq!(refetched.request.body.mode_name(), "json");

    let dup = repo::duplicate_endpoint(&db, created.id).await.unwrap();
    assert_ne!(dup.id, created.id);
    assert_eq!(dup.name, format!("{}（副本）", created.name));
    assert_eq!(dup.path, "/users");

    let listed = repo::list_endpoints(&db, project.id).await.unwrap();
    assert_eq!(listed.len(), 2);

    repo::delete_endpoint(&db, created.id).await.unwrap();
    assert!(repo::get_endpoint(&db, created.id).await.is_err());
    assert_eq!(
        repo::list_endpoints(&db, project.id).await.unwrap().len(),
        1
    );
}

#[tokio::test]
async fn environment_crud() {
    let db = pool().await;
    let project = repo::create_project(&db, "P", "").await.unwrap();

    let env = repo::create_environment(&db, project.id, "local", &HashMap::new())
        .await
        .unwrap();
    assert_eq!(env.name, "local");

    let mut updated = env.clone();
    updated.variables.insert(
        "base_url".into(),
        "https://jsonplaceholder.typicode.com".into(),
    );
    updated.variables.insert("token".into(), "abc".into());
    repo::update_environment(&db, &updated).await.unwrap();

    let fetched = repo::get_environment(&db, env.id).await.unwrap();
    assert_eq!(
        fetched.variables["base_url"],
        "https://jsonplaceholder.typicode.com"
    );
    assert_eq!(fetched.variables.len(), 2);

    let listed = repo::list_environments(&db, project.id).await.unwrap();
    assert_eq!(listed.len(), 1);

    // M11：落库应为密文（不包含明文 token / 键名），且不含加密格式前缀（明文容错路径）
    let raw: (String,) = sqlx::query_as("SELECT variables_json FROM environments WHERE id = ?")
        .bind(env.id.to_string())
        .fetch_one(&db)
        .await
        .unwrap();
    assert!(
        !raw.0.contains("abc"),
        "变量应加密存储，明文出现在库中: {}",
        raw.0
    );
    assert!(raw.0.contains(':'), "密文应为 base64:base64 格式");

    repo::delete_environment(&db, env.id).await.unwrap();
    assert!(repo::list_environments(&db, project.id)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn cascade_delete_project() {
    let db = pool().await;
    let project = repo::create_project(&db, "P", "").await.unwrap();
    let folder = repo::create_folder(&db, project.id, None, "F")
        .await
        .unwrap();
    let ep = repo::create_endpoint(&db, project.id, Some(folder.id), "E")
        .await
        .unwrap();
    let env = repo::create_environment(&db, project.id, "E", &HashMap::new())
        .await
        .unwrap();
    assert_eq!(
        repo::list_endpoints(&db, project.id).await.unwrap().len(),
        1
    );

    let other = repo::create_project(&db, "Q", "").await.unwrap();
    repo::create_endpoint(&db, other.id, None, "X")
        .await
        .unwrap();

    repo::delete_project(&db, project.id).await.unwrap();

    assert!(repo::get_endpoint(&db, ep.id).await.is_err());
    assert!(repo::get_folder(&db, folder.id).await.is_err());
    assert!(repo::get_environment(&db, env.id).await.is_err());
    assert_eq!(repo::list_projects(&db).await.unwrap().len(), 1);
}

#[tokio::test]
async fn settings_roundtrip() {
    let db = pool().await;
    assert!(repo::get_setting(&db, "k").await.unwrap().is_none());
    repo::set_setting(&db, "port", "4010").await.unwrap();
    assert_eq!(
        repo::get_setting(&db, "port").await.unwrap(),
        Some("4010".into())
    );
    repo::set_setting(&db, "port", "4011").await.unwrap();
    assert_eq!(
        repo::get_setting(&db, "port").await.unwrap(),
        Some("4011".into())
    );
}

#[tokio::test]
async fn ws_message_enqueue_list_delete() {
    let db = pool().await;

    repo::enqueue_ws_message(&db, "ws://a", WsMessageType::Text, "hello")
        .await
        .unwrap();
    repo::enqueue_ws_message(&db, "ws://a", WsMessageType::Binary, "AQID")
        .await
        .unwrap();
    // 其它目标地址互不影响。
    repo::enqueue_ws_message(&db, "ws://b", WsMessageType::Ping, "p1")
        .await
        .unwrap();

    let list = repo::list_pending_ws_messages(&db, "ws://a").await.unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].message_type, WsMessageType::Text);
    assert_eq!(list[0].payload, "hello");
    assert_eq!(list[1].message_type, WsMessageType::Binary);
    assert_eq!(list[1].payload, "AQID");

    repo::delete_ws_messages(&db, &[list[0].id]).await.unwrap();
    let after = repo::list_pending_ws_messages(&db, "ws://a").await.unwrap();
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].id, list[1].id);
    assert_eq!(
        repo::list_pending_ws_messages(&db, "ws://b")
            .await
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn ws_message_purges_expired() {
    let db = pool().await;
    let record = repo::enqueue_ws_message(&db, "ws://a", WsMessageType::Text, "old")
        .await
        .unwrap();
    // 把记录改到 48 小时前，模拟过期消息。
    let old = (chrono::Utc::now() - chrono::Duration::hours(48)).to_rfc3339();
    sqlx::query("UPDATE ws_messages SET created_at = ? WHERE id = ?")
        .bind(old)
        .bind(record.id.to_string())
        .execute(&db)
        .await
        .unwrap();

    // 24 小时内的新消息不受影响。
    repo::enqueue_ws_message(&db, "ws://a", WsMessageType::Text, "fresh")
        .await
        .unwrap();

    let removed = repo::purge_expired_ws_messages(&db, "ws://a", chrono::Duration::hours(24))
        .await
        .unwrap();
    assert_eq!(removed, 1);
    let list = repo::list_pending_ws_messages(&db, "ws://a").await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].payload, "fresh");
}

#[tokio::test]
async fn save_folder_repeated_id_updates_not_conflicts() {
    let db = pool().await;
    let project = repo::create_project(&db, "P", "").await.unwrap();

    let created = repo::create_folder(&db, project.id, None, "原名字")
        .await
        .unwrap();
    let mut renamed = created.clone();
    renamed.name = "新名字".into();
    renamed.updated_at = chrono::Utc::now();
    // 重命名走 save_*（带 id 再次保存），此前因主键冲突失败，回归此问题。
    repo::save_folder(&db, &renamed).await.unwrap();

    let fetched = repo::get_folder(&db, created.id).await.unwrap();
    assert_eq!(fetched.name, "新名字");
}

#[tokio::test]
async fn save_project_repeated_id_updates_not_conflicts() {
    let db = pool().await;
    let created = repo::create_project(&db, "原名", "").await.unwrap();
    let mut renamed = created.clone();
    renamed.name = "改名".into();
    renamed.updated_at = chrono::Utc::now();
    repo::save_project(&db, &renamed).await.unwrap();

    let fetched = repo::get_project(&db, created.id).await.unwrap();
    assert_eq!(fetched.name, "改名");
}

#[tokio::test]
async fn save_environment_repeated_id_updates_not_conflicts() {
    let db = pool().await;
    let project = repo::create_project(&db, "P", "").await.unwrap();
    let created = repo::create_environment(&db, project.id, "开发", &HashMap::new()).await.unwrap();
    let mut edited = created.clone();
    edited.name = "生产".into();
    edited.updated_at = chrono::Utc::now();
    repo::save_environment(&db, &edited).await.unwrap();

    let fetched = repo::get_environment(&db, created.id).await.unwrap();
    assert_eq!(fetched.name, "生产");
}
