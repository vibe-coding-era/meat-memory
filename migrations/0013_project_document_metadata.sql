ALTER TABLE project_documents
  ADD COLUMN IF NOT EXISTS metadata JSONB NOT NULL DEFAULT '{}'::jsonb;

CREATE INDEX IF NOT EXISTS idx_project_documents_metadata_gin
  ON project_documents USING GIN (metadata);
