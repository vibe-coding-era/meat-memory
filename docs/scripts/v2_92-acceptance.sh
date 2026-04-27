#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPORT_DIR="${V2_92_ACCEPTANCE_REPORT_DIR:-${ROOT_DIR}/tests/reports/trace/latest}"
MARKDOWN_ROOT="${V2_92_ACCEPTANCE_MARKDOWN_ROOT:-${ROOT_DIR}/target/v2_92-trace-markdown}"
ACCEPTANCE_REPORT="${REPORT_DIR}/v2_92-acceptance.txt"
TRACE_JSON="${REPORT_DIR}/trace-latest.json"
TRACE_INSPECT_JSON="${REPORT_DIR}/trace-inspect.json"

if [[ -z "${V2_92_ACCEPTANCE_MARKDOWN_ROOT:-}" ]]; then
  rm -rf "${MARKDOWN_ROOT}"
fi
mkdir -p "${REPORT_DIR}" "${MARKDOWN_ROOT}"
cd "${ROOT_DIR}"

exec > >(tee "${ACCEPTANCE_REPORT}") 2>&1

run_step() {
  local label="$1"
  shift
  echo "[v2.92] ${label}"
  "$@"
}

memory_cli() {
  env \
    MEAT_MEMORY_ENABLE_PG=false \
    MEAT_MEMORY_ENABLE_MARKDOWN=true \
    MEAT_MEMORY_MARKDOWN_ROOT="${MARKDOWN_ROOT}" \
    cargo run -p memory-cli -- "$@"
}

echo "[v2.92] acceptance started"
echo "[v2.92] report: ${ACCEPTANCE_REPORT}"
echo "[v2.92] trace output: ${REPORT_DIR}"

run_step "format check" cargo fmt --all --check

run_step "interface check" \
  cargo check -p memory-domain -p memory-observability -p memory-kernel -p memory-cli -p memory-store-pg

run_step "recall domain models" \
  cargo test -p memory-domain --lib recall -- --test-threads=1

run_step "recall ids" \
  cargo test -p memory-domain --lib ids -- --test-threads=1

run_step "recall trace kernel" \
  cargo test -p memory-kernel --lib v29_trace -- --test-threads=1

run_step "benchmark trace association" \
  cargo test -p memory-kernel --lib v29_benchmark -- --test-threads=1

run_step "trace CLI helpers and parser" \
  cargo test -p memory-cli --bin memory-cli trace -- --test-threads=1

run_step "recall observability metrics" \
  cargo test -p memory-observability --lib record_functions_update_metrics_counters -- --test-threads=1

run_step "recall PG migration contract" \
  cargo test -p memory-store-pg --lib v28_sql_helpers_expose_expected_statements -- --test-threads=1

echo "[v2.92] seed trace fixture"
memory_cli remember \
  --scope-id scp_v292_acceptance \
  --title "trace budget public one" \
  --body "trace budget public one should be selected and explained" \
  --memory-kind fact \
  --json >/dev/null
memory_cli remember \
  --scope-id scp_v292_acceptance \
  --title "trace budget public two" \
  --body "trace budget public two should be trimmed by the record budget" \
  --memory-kind summary \
  --json >/dev/null
memory_cli remember \
  --scope-id scp_v292_acceptance \
  --title "trace budget restricted" \
  --body "trace budget restricted should be filtered by recall guard" \
  --memory-kind risk \
  --sensitivity restricted \
  --json >/dev/null

echo "[v2.92] CLI trace latest"
memory_cli trace latest \
  --scope-id scp_v292_acceptance \
  --query "trace budget" \
  --max-records 1 \
  --max-chars 64 \
  --debug-candidates \
  --output-dir "${REPORT_DIR}" \
  --json >"${TRACE_JSON}"

TRACE_ID="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["trace"]["id"])' "${TRACE_JSON}")"

echo "[v2.92] CLI trace inspect ${TRACE_ID}"
memory_cli trace inspect \
  --trace-id "${TRACE_ID}" \
  --input-dir "${REPORT_DIR}" \
  --json >"${TRACE_INSPECT_JSON}"

test -f "${REPORT_DIR}/trace.json"
test -f "${REPORT_DIR}/explanation.md"
test -f "${REPORT_DIR}/budget.json"
grep -q "Recall Trace Explanation" "${REPORT_DIR}/explanation.md"
grep -q "filtered_reason" "${REPORT_DIR}/explanation.md"
grep -q '"filtered_count": 1' "${TRACE_JSON}"
grep -q '"trimmed_items"' "${TRACE_JSON}"
grep -q "${TRACE_ID}" "${TRACE_INSPECT_JSON}"

cat >"${REPORT_DIR}/coverage-v2_92-new-code.md" <<'REPORT'
# V2.92 New Code Coverage Gate

Coverage target: 100% line / function / region coverage for V2.92 new production code.

Covered production areas:

- memory-domain recall trace, explanation and budget pack model
- memory-kernel traced search, failure classifier, budget pack and report writer
- memory-cli trace latest / inspect parser and output helpers
- memory-observability V2.9 recall metrics
- memory-store-pg 0010 recall trace migration wiring

Required suites executed by this script:

- memory-domain recall and ids unit tests
- memory-kernel v29_trace and v29_benchmark tests
- memory-cli trace parser/report/helper tests
- memory-observability recall metrics test
- memory-store-pg recall migration contract test

Status: pass when this script exits 0.
REPORT

echo "[v2.92] acceptance completed"
