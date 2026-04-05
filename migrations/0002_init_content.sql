CREATE TABLE IF NOT EXISTS episodes (
  id TEXT PRIMARY KEY,
  scope_id TEXT NOT NULL REFERENCES scopes(id) ON DELETE CASCADE,
  episode_kind TEXT NOT NULL,
  state TEXT NOT NULL,
  title TEXT NOT NULL,
  summary TEXT,
  started_at TIMESTAMPTZ NOT NULL,
  ended_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS artifacts (
  id TEXT PRIMARY KEY,
  scope_id TEXT NOT NULL REFERENCES scopes(id) ON DELETE CASCADE,
  artifact_kind TEXT NOT NULL,
  mime_type TEXT,
  language_code TEXT,
  content_text TEXT NOT NULL,
  content_hash TEXT NOT NULL,
  visibility TEXT NOT NULL,
  sensitivity TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL,
  updated_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_artifacts_scope_id ON artifacts(scope_id);
CREATE INDEX IF NOT EXISTS idx_artifacts_content_hash ON artifacts(content_hash);

CREATE TABLE IF NOT EXISTS memories (
  id TEXT PRIMARY KEY,
  scope_id TEXT NOT NULL REFERENCES scopes(id) ON DELETE CASCADE,
  memory_kind TEXT NOT NULL,
  state TEXT NOT NULL,
  title TEXT NOT NULL,
  body TEXT NOT NULL,
  confidence REAL NOT NULL,
  importance REAL NOT NULL,
  stability REAL NOT NULL,
  freshness REAL NOT NULL,
  visibility TEXT NOT NULL,
  sensitivity TEXT NOT NULL,
  evidence_count INTEGER NOT NULL DEFAULT 0,
  created_at TIMESTAMPTZ NOT NULL,
  updated_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_memories_scope_id ON memories(scope_id);
CREATE INDEX IF NOT EXISTS idx_memories_title ON memories(title);

CREATE TABLE IF NOT EXISTS memory_versions (
  memory_id TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
  version INTEGER NOT NULL,
  title TEXT NOT NULL,
  body TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  PRIMARY KEY (memory_id, version)
);

CREATE TABLE IF NOT EXISTS memory_evidence_links (
  memory_id TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
  artifact_id TEXT NOT NULL REFERENCES artifacts(id) ON DELETE CASCADE,
  quote_text TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  PRIMARY KEY (memory_id, artifact_id)
);

CREATE INDEX IF NOT EXISTS idx_memory_evidence_links_artifact_id ON memory_evidence_links(artifact_id);
