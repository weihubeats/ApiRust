CREATE TABLE IF NOT EXISTS request_examples (
    id TEXT PRIMARY KEY,
    endpoint_id TEXT NOT NULL REFERENCES endpoints(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    request_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_request_examples_endpoint ON request_examples(endpoint_id);