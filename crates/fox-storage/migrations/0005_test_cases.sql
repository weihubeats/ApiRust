CREATE TABLE IF NOT EXISTS test_cases (
    id TEXT PRIMARY KEY NOT NULL,
    request_id TEXT NOT NULL REFERENCES endpoints(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    category TEXT NOT NULL DEFAULT '正向',
    method TEXT NOT NULL,
    url_path TEXT NOT NULL DEFAULT '/',
    params TEXT,
    headers TEXT,
    body_type TEXT NOT NULL DEFAULT 'none',
    body_content TEXT NOT NULL DEFAULT '',
    last_run_status TEXT NOT NULL DEFAULT 'Untested',
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_test_cases_request ON test_cases(request_id);