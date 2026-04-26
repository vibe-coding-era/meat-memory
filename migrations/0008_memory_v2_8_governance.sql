ALTER TABLE memory_versions
  ADD COLUMN IF NOT EXISTS change_kind TEXT NOT NULL DEFAULT 'upsert',
  ADD COLUMN IF NOT EXISTS actor TEXT NOT NULL DEFAULT 'system',
  ADD COLUMN IF NOT EXISTS reason TEXT,
  ADD COLUMN IF NOT EXISTS source_proposal_id TEXT;

CREATE TABLE IF NOT EXISTS project_identity_bindings (
  id TEXT PRIMARY KEY,
  owner_scope_id TEXT NOT NULL REFERENCES scopes(id) ON DELETE CASCADE,
  binding_kind TEXT NOT NULL,
  binding_value TEXT NOT NULL,
  canonical_project_key TEXT NOT NULL,
  confirmed_by TEXT NOT NULL,
  confidence REAL NOT NULL,
  created_at TIMESTAMPTZ NOT NULL,
  updated_at TIMESTAMPTZ NOT NULL,
  UNIQUE (owner_scope_id, binding_kind, binding_value)
);

CREATE INDEX IF NOT EXISTS idx_project_identity_bindings_owner_updated
  ON project_identity_bindings(owner_scope_id, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_project_identity_bindings_canonical_key
  ON project_identity_bindings(canonical_project_key);

CREATE TABLE IF NOT EXISTS memory_proposals (
  id TEXT PRIMARY KEY,
  scope_id TEXT NOT NULL REFERENCES scopes(id) ON DELETE CASCADE,
  proposal_type TEXT NOT NULL,
  status TEXT NOT NULL,
  review_level TEXT NOT NULL,
  subject_memory_id TEXT REFERENCES memories(id) ON DELETE SET NULL,
  target_memory_ids JSONB NOT NULL DEFAULT '[]'::jsonb,
  reason TEXT NOT NULL,
  evidence JSONB NOT NULL DEFAULT '[]'::jsonb,
  decided_by TEXT,
  decided_at TIMESTAMPTZ,
  applied_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL,
  updated_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_memory_proposals_scope_status_updated
  ON memory_proposals(scope_id, status, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_memory_proposals_subject
  ON memory_proposals(subject_memory_id);

CREATE TABLE IF NOT EXISTS memory_relations (
  id TEXT PRIMARY KEY,
  scope_id TEXT NOT NULL REFERENCES scopes(id) ON DELETE CASCADE,
  from_memory_id TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
  to_memory_id TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
  relation_type TEXT NOT NULL,
  confidence REAL NOT NULL,
  source_kind TEXT NOT NULL,
  source_proposal_id TEXT REFERENCES memory_proposals(id) ON DELETE SET NULL,
  created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_memory_relations_scope_from_created
  ON memory_relations(scope_id, from_memory_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_memory_relations_scope_to_created
  ON memory_relations(scope_id, to_memory_id, created_at DESC);

CREATE TABLE IF NOT EXISTS distillation_profiles (
  id TEXT PRIMARY KEY,
  scope_id TEXT REFERENCES scopes(id) ON DELETE CASCADE,
  profile_level TEXT NOT NULL,
  status TEXT NOT NULL,
  name TEXT NOT NULL,
  prompt_text TEXT NOT NULL,
  rules_json JSONB NOT NULL DEFAULT '{}'::jsonb,
  focus_topics JSONB NOT NULL DEFAULT '[]'::jsonb,
  prefer_memory_kinds JSONB NOT NULL DEFAULT '[]'::jsonb,
  created_by TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL,
  updated_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_distillation_profiles_scope_status_updated
  ON distillation_profiles(scope_id, status, updated_at DESC);

CREATE TABLE IF NOT EXISTS distillation_runs (
  id TEXT PRIMARY KEY,
  profile_id TEXT REFERENCES distillation_profiles(id) ON DELETE SET NULL,
  scope_id TEXT NOT NULL REFERENCES scopes(id) ON DELETE CASCADE,
  input_hash TEXT NOT NULL,
  preview BOOLEAN NOT NULL,
  output_json JSONB NOT NULL DEFAULT '{}'::jsonb,
  created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_distillation_runs_scope_created
  ON distillation_runs(scope_id, created_at DESC);
