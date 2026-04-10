ALTER TABLE artifacts
  ADD COLUMN IF NOT EXISTS labels JSONB NOT NULL DEFAULT '[]'::jsonb;

ALTER TABLE memories
  ADD COLUMN IF NOT EXISTS owner_scope_id TEXT REFERENCES scopes(id) ON DELETE SET NULL,
  ADD COLUMN IF NOT EXISTS published_from_scope_id TEXT REFERENCES scopes(id) ON DELETE SET NULL,
  ADD COLUMN IF NOT EXISTS language_code TEXT;

UPDATE memories
SET owner_scope_id = scope_id
WHERE owner_scope_id IS NULL;
