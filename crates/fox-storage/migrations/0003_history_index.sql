-- 历史列表查询按项目过滤 + 时间倒序，补复合索引避免全量排序
CREATE INDEX IF NOT EXISTS idx_histories_project_created
    ON request_histories (project_id, created_at DESC);

-- 一次性裁剪存量：每项目仅保留最新 500 条（与 HISTORY_RETENTION_PER_PROJECT 一致）
DELETE FROM request_histories
WHERE id IN (
    SELECT id FROM request_histories h
    WHERE (
        SELECT COUNT(*) FROM request_histories h2
        WHERE h2.project_id = h.project_id AND h2.created_at > h.created_at
    ) >= 500
);
