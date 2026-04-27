CREATE TABLE IF NOT EXISTS recall_traces (
    id TEXT PRIMARY KEY,
    scope_id TEXT NOT NULL,
    query TEXT NOT NULL,
    retrieval_mode TEXT NOT NULL,
    candidate_count INTEGER NOT NULL DEFAULT 0,
    selected_count INTEGER NOT NULL DEFAULT 0,
    filtered_count INTEGER NOT NULL DEFAULT 0,
    budget_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    retention TEXT NOT NULL DEFAULT 'summary_only',
    explanation_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS recall_trace_candidates (
    trace_id TEXT NOT NULL REFERENCES recall_traces(id) ON DELETE CASCADE,
    memory_id TEXT NOT NULL,
    title TEXT NOT NULL,
    rank INTEGER NOT NULL,
    score BIGINT NOT NULL DEFAULT 0,
    selected BOOLEAN NOT NULL DEFAULT FALSE,
    filtered_reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (trace_id, memory_id)
);

CREATE TABLE IF NOT EXISTS recall_budget_packs (
    id TEXT PRIMARY KEY,
    trace_id TEXT NOT NULL REFERENCES recall_traces(id) ON DELETE CASCADE,
    max_records INTEGER NOT NULL DEFAULT 0,
    max_chars INTEGER NOT NULL DEFAULT 0,
    used_chars INTEGER NOT NULL DEFAULT 0,
    items_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    trimmed_items_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_recall_traces_scope_created
    ON recall_traces (scope_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_recall_trace_candidates_selected
    ON recall_trace_candidates (trace_id, selected);
