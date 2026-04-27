CREATE TABLE IF NOT EXISTS secret_findings (
    id TEXT PRIMARY KEY,
    scope_id TEXT NOT NULL REFERENCES scopes(id) ON DELETE CASCADE,
    memory_id TEXT REFERENCES memories(id) ON DELETE SET NULL,
    source_ref TEXT NOT NULL,
    kind TEXT NOT NULL,
    action TEXT NOT NULL,
    risk_level TEXT NOT NULL,
    fingerprint TEXT NOT NULL,
    redacted_preview TEXT NOT NULL,
    location_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS memory_health_reports (
    id TEXT PRIMARY KEY,
    scope_id TEXT REFERENCES scopes(id) ON DELETE CASCADE,
    total_count INT NOT NULL DEFAULT 0,
    risk_count INT NOT NULL DEFAULT 0,
    secret_finding_count INT NOT NULL DEFAULT 0,
    high_risk_secret_finding_count INT NOT NULL DEFAULT 0,
    report_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_secret_findings_scope_created
    ON secret_findings (scope_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_secret_findings_memory
    ON secret_findings (memory_id);

CREATE INDEX IF NOT EXISTS idx_memory_health_reports_scope_created
    ON memory_health_reports (scope_id, created_at DESC);
