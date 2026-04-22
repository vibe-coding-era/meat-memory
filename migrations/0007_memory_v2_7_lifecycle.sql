ALTER TABLE memories
  ADD COLUMN IF NOT EXISTS source_refs JSONB NOT NULL DEFAULT '[]'::jsonb;

CREATE TABLE IF NOT EXISTS memory_lifecycle_audit_events (
  id TEXT PRIMARY KEY,
  scope_id TEXT NOT NULL,
  memory_id TEXT NOT NULL,
  action TEXT NOT NULL,
  actor TEXT NOT NULL,
  before_status TEXT,
  after_status TEXT,
  reason TEXT,
  created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_memory_lifecycle_audit_scope_created
  ON memory_lifecycle_audit_events(scope_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_memory_lifecycle_audit_memory_created
  ON memory_lifecycle_audit_events(memory_id, created_at DESC);
