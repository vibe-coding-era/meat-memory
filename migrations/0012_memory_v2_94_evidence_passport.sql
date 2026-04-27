CREATE TABLE IF NOT EXISTS evidence_spans (
    id TEXT PRIMARY KEY,
    scope_id TEXT NOT NULL REFERENCES scopes(id) ON DELETE CASCADE,
    memory_id TEXT REFERENCES memories(id) ON DELETE SET NULL,
    artifact_id TEXT REFERENCES artifacts(id) ON DELETE SET NULL,
    source_ref TEXT NOT NULL,
    kind TEXT NOT NULL,
    quote TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    location_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS memory_passport_exports (
    id TEXT PRIMARY KEY,
    scope_id TEXT NOT NULL REFERENCES scopes(id) ON DELETE CASCADE,
    schema_version TEXT NOT NULL,
    object_count INT NOT NULL DEFAULT 0,
    redaction TEXT NOT NULL,
    encryption_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    bundle_hash TEXT NOT NULL,
    manifest_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_evidence_spans_scope_memory
    ON evidence_spans (scope_id, memory_id);

CREATE INDEX IF NOT EXISTS idx_evidence_spans_artifact
    ON evidence_spans (artifact_id);

CREATE INDEX IF NOT EXISTS idx_memory_passport_exports_scope_created
    ON memory_passport_exports (scope_id, created_at DESC);
