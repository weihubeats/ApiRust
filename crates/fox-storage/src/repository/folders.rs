//! Folder CRUD。

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use fox_core::model::Folder;
use fox_core::{AppError, Result};

use super::rows::FolderRow;

pub async fn create_folder(
    db: &SqlitePool,
    project_id: Uuid,
    parent_id: Option<Uuid>,
    name: &str,
) -> Result<Folder> {
    let now = Utc::now();
    let model = Folder {
        id: Uuid::new_v4(),
        project_id,
        parent_id,
        name: name.to_string(),
        sort_order: 0,
        created_at: now,
        updated_at: now,
    };
    let row = FolderRow::from_model(&model);
    sqlx::query(
        "INSERT INTO folders (id, project_id, parent_id, name, sort_order, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&row.id)
    .bind(&row.project_id)
    .bind(&row.parent_id)
    .bind(&row.name)
    .bind(row.sort_order)
    .bind(row.created_at.clone())
    .bind(row.updated_at.clone())
    .execute(db)
    .await?;
    Ok(model)
}

pub async fn list_folders(db: &SqlitePool, project_id: Uuid) -> Result<Vec<Folder>> {
    let rows: Vec<FolderRow> = sqlx::query_as(
        "SELECT id, project_id, parent_id, name, sort_order, created_at, updated_at
         FROM folders WHERE project_id = ? ORDER BY sort_order, created_at",
    )
    .bind(project_id.to_string())
    .fetch_all(db)
    .await?;
    rows.into_iter().map(FolderRow::into_model).collect()
}

pub async fn get_folder(db: &SqlitePool, folder_id: Uuid) -> Result<Folder> {
    let row: Option<FolderRow> = sqlx::query_as(
        "SELECT id, project_id, parent_id, name, sort_order, created_at, updated_at
         FROM folders WHERE id = ?",
    )
    .bind(folder_id.to_string())
    .fetch_optional(db)
    .await?;
    row.map(FolderRow::into_model)
        .transpose()?
        .ok_or_else(|| AppError::NotFound(format!("文件夹（{folder_id}）")))
}

pub async fn update_folder(db: &SqlitePool, folder: &Folder) -> Result<Folder> {
    let row = FolderRow::from_model(folder);
    sqlx::query(
        "UPDATE folders SET parent_id = ?, name = ?, sort_order = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(&row.parent_id)
    .bind(&row.name)
    .bind(row.sort_order)
    .bind(row.updated_at.clone())
    .bind(&row.id)
    .execute(db)
    .await?;
    Ok(folder.clone())
}

/// 递归收集某文件夹的整个子树（含自身）的 CTE 前缀。
///
/// folders.parent_id / endpoints.folder_id 外键均为 `ON DELETE SET NULL`，
/// 直接删除父文件夹会留下「孤儿」子文件夹与接口，因此删除时用该 CTE
/// 显式收集全部后代并级联清理。
const FOLDER_SUBTREE_SQL: &str = "WITH RECURSIVE subtree(id) AS (
    SELECT ?
    UNION ALL
    SELECT f.id FROM folders f JOIN subtree s ON f.parent_id = s.id
)";

/// 删除文件夹及其全部子孙文件夹、子孙文件夹下的接口（事务内级联）。
pub async fn delete_folder(db: &SqlitePool, folder_id: Uuid) -> Result<()> {
    let id = folder_id.to_string();
    let mut tx = db.begin().await?;

    // 先清掉子树下全部接口（外键为 SET NULL，不会自动级联删除）。
    sqlx::query(&format!(
        "{FOLDER_SUBTREE_SQL} DELETE FROM endpoints WHERE folder_id IN (SELECT id FROM subtree)"
    ))
    .bind(&id)
    .execute(&mut *tx)
    .await?;

    // 再递归删除子树全部文件夹。
    let affected = sqlx::query(&format!(
        "{FOLDER_SUBTREE_SQL} DELETE FROM folders WHERE id IN (SELECT id FROM subtree)"
    ))
    .bind(&id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    if affected.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("文件夹（{folder_id}）")));
    }
    Ok(())
}

/// 带 id：原样写入文件夹（upsert，同一 id 重复保存时更新而非报主键冲突）。
pub async fn save_folder(db: &SqlitePool, folder: &Folder) -> Result<()> {
    let row = FolderRow::from_model(folder);
    sqlx::query(
        "INSERT INTO folders (id, project_id, parent_id, name, sort_order, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
            project_id = excluded.project_id,
            parent_id = excluded.parent_id,
            name = excluded.name,
            sort_order = excluded.sort_order,
            updated_at = excluded.updated_at",
    )
    .bind(&row.id)
    .bind(&row.project_id)
    .bind(&row.parent_id)
    .bind(&row.name)
    .bind(row.sort_order)
    .bind(row.created_at.clone())
    .bind(row.updated_at.clone())
    .execute(db)
    .await?;
    Ok(())
}
