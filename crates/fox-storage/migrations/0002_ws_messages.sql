-- WebSocket 离线消息：发送队列溢出 / 连接不可达时持久化，重连成功后补发。
CREATE TABLE IF NOT EXISTS ws_messages (
    id TEXT PRIMARY KEY,
    target TEXT NOT NULL,
    message_type TEXT NOT NULL,
    payload TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_ws_messages_target_created ON ws_messages(target, created_at);