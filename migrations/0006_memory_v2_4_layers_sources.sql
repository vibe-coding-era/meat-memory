CREATE TABLE IF NOT EXISTS memory_sources (
  id TEXT PRIMARY KEY,
  source_kind TEXT NOT NULL,
  display_name TEXT NOT NULL,
  owner_principal_id TEXT NOT NULL,
  owner_scope_id TEXT NOT NULL REFERENCES scopes(id) ON DELETE CASCADE,
  source_uri TEXT,
  sync_mode TEXT NOT NULL DEFAULT 'read_only',
  local_root TEXT,
  status TEXT NOT NULL DEFAULT 'active',
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_memory_sources_owner_scope_id ON memory_sources(owner_scope_id);
CREATE INDEX IF NOT EXISTS idx_memory_sources_source_kind ON memory_sources(source_kind);
CREATE INDEX IF NOT EXISTS idx_memory_sources_status ON memory_sources(status);

ALTER TABLE access_keys
  ADD COLUMN IF NOT EXISTS source_id TEXT REFERENCES memory_sources(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_access_keys_source_id ON access_keys(source_id);

CREATE TABLE IF NOT EXISTS agent_contexts (
  id TEXT PRIMARY KEY,
  source_id TEXT REFERENCES memory_sources(id) ON DELETE SET NULL,
  key_id TEXT REFERENCES access_keys(id) ON DELETE SET NULL,
  scope_id TEXT NOT NULL REFERENCES scopes(id) ON DELETE CASCADE,
  session_id TEXT NOT NULL,
  task_id TEXT,
  layer TEXT NOT NULL DEFAULT 'short_term',
  title TEXT NOT NULL,
  body TEXT NOT NULL,
  labels JSONB NOT NULL DEFAULT '[]'::jsonb,
  expires_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_agent_contexts_scope_id ON agent_contexts(scope_id);
CREATE INDEX IF NOT EXISTS idx_agent_contexts_source_id ON agent_contexts(source_id);
CREATE INDEX IF NOT EXISTS idx_agent_contexts_key_id ON agent_contexts(key_id);
CREATE INDEX IF NOT EXISTS idx_agent_contexts_session_id ON agent_contexts(session_id);
CREATE INDEX IF NOT EXISTS idx_agent_contexts_task_id ON agent_contexts(task_id);
CREATE INDEX IF NOT EXISTS idx_agent_contexts_expires_at ON agent_contexts(expires_at);

CREATE TABLE IF NOT EXISTS project_documents (
  id TEXT PRIMARY KEY,
  source_id TEXT NOT NULL REFERENCES memory_sources(id) ON DELETE CASCADE,
  scope_id TEXT NOT NULL REFERENCES scopes(id) ON DELETE CASCADE,
  local_path TEXT,
  canonical_uri TEXT NOT NULL,
  title TEXT NOT NULL,
  content_hash TEXT NOT NULL,
  last_seen_mtime TIMESTAMPTZ,
  sync_state TEXT NOT NULL DEFAULT 'clean',
  conflict_state TEXT NOT NULL DEFAULT 'none',
  artifact_id TEXT REFERENCES artifacts(id) ON DELETE SET NULL,
  memory_id TEXT REFERENCES memories(id) ON DELETE SET NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE (source_id, canonical_uri)
);

CREATE INDEX IF NOT EXISTS idx_project_documents_scope_id ON project_documents(scope_id);
CREATE INDEX IF NOT EXISTS idx_project_documents_source_id ON project_documents(source_id);
CREATE INDEX IF NOT EXISTS idx_project_documents_canonical_uri ON project_documents(canonical_uri);
CREATE INDEX IF NOT EXISTS idx_project_documents_sync_state ON project_documents(sync_state);
CREATE INDEX IF NOT EXISTS idx_project_documents_conflict_state ON project_documents(conflict_state);
