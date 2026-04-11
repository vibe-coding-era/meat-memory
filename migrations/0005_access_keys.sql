CREATE TABLE IF NOT EXISTS access_keys (
  id TEXT PRIMARY KEY,
  key_hash TEXT NOT NULL UNIQUE,
  display_name TEXT NOT NULL,
  source_kind TEXT NOT NULL,
  owner_principal_id TEXT NOT NULL,
  owner_scope_id TEXT NOT NULL REFERENCES scopes(id) ON DELETE CASCADE,
  scope_kind TEXT NOT NULL,
  storage_mode TEXT NOT NULL,
  is_fully_isolated BOOLEAN NOT NULL DEFAULT false,
  isolation_group_id TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'active',
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  last_used_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_access_keys_owner_scope_id ON access_keys(owner_scope_id);
CREATE INDEX IF NOT EXISTS idx_access_keys_isolation_group_id ON access_keys(isolation_group_id);
CREATE INDEX IF NOT EXISTS idx_access_keys_source_kind ON access_keys(source_kind);

CREATE TABLE IF NOT EXISTS key_usage_events (
  id TEXT PRIMARY KEY,
  key_id TEXT REFERENCES access_keys(id) ON DELETE SET NULL,
  source_kind TEXT NOT NULL,
  operation TEXT NOT NULL,
  scope_id TEXT,
  storage_mode TEXT NOT NULL,
  success BOOLEAN NOT NULL,
  latency_ms BIGINT NOT NULL DEFAULT 0,
  error_code TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_key_usage_events_key_id ON key_usage_events(key_id);
CREATE INDEX IF NOT EXISTS idx_key_usage_events_created_at ON key_usage_events(created_at);
CREATE INDEX IF NOT EXISTS idx_key_usage_events_operation ON key_usage_events(operation);

CREATE TABLE IF NOT EXISTS memory_key_links (
  memory_id TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
  key_id TEXT NOT NULL REFERENCES access_keys(id) ON DELETE CASCADE,
  isolation_group_id TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  PRIMARY KEY (memory_id, key_id)
);

CREATE INDEX IF NOT EXISTS idx_memory_key_links_key_id ON memory_key_links(key_id);
CREATE INDEX IF NOT EXISTS idx_memory_key_links_isolation_group_id ON memory_key_links(isolation_group_id);

CREATE TABLE IF NOT EXISTS memory_embeddings (
  memory_id TEXT PRIMARY KEY REFERENCES memories(id) ON DELETE CASCADE,
  key_id TEXT REFERENCES access_keys(id) ON DELETE SET NULL,
  isolation_group_id TEXT NOT NULL,
  embedding_model_alias TEXT NOT NULL,
  embedding vector(1536),
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_memory_embeddings_key_id ON memory_embeddings(key_id);
CREATE INDEX IF NOT EXISTS idx_memory_embeddings_isolation_group_id ON memory_embeddings(isolation_group_id);
