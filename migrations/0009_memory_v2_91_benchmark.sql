CREATE TABLE IF NOT EXISTS benchmark_suites (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    metric_profile TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (name, version)
);

CREATE TABLE IF NOT EXISTS benchmark_runs (
    id TEXT PRIMARY KEY,
    suite_id TEXT NOT NULL REFERENCES benchmark_suites(id) ON DELETE CASCADE,
    suite_name TEXT NOT NULL,
    status TEXT NOT NULL,
    started_at TIMESTAMPTZ NOT NULL,
    finished_at TIMESTAMPTZ,
    case_count INTEGER NOT NULL DEFAULT 0,
    metrics_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    report_path TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS benchmark_case_results (
    run_id TEXT NOT NULL REFERENCES benchmark_runs(id) ON DELETE CASCADE,
    case_id TEXT NOT NULL,
    query TEXT NOT NULL,
    expected_memory_titles TEXT[] NOT NULL DEFAULT '{}',
    actual_memory_ids TEXT[] NOT NULL DEFAULT '{}',
    actual_memory_titles TEXT[] NOT NULL DEFAULT '{}',
    recall_at_1 BOOLEAN NOT NULL DEFAULT FALSE,
    recall_at_5 BOOLEAN NOT NULL DEFAULT FALSE,
    latency_ms BIGINT NOT NULL DEFAULT 0,
    leakage BOOLEAN NOT NULL DEFAULT FALSE,
    failure_reason TEXT,
    result_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (run_id, case_id)
);

CREATE INDEX IF NOT EXISTS idx_benchmark_runs_suite_started
    ON benchmark_runs (suite_id, started_at DESC);

CREATE INDEX IF NOT EXISTS idx_benchmark_case_results_failure
    ON benchmark_case_results (run_id)
    WHERE failure_reason IS NOT NULL OR leakage = TRUE;
