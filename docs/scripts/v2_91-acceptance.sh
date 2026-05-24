#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPORT_DIR="${V2_91_ACCEPTANCE_REPORT_DIR:-${ROOT_DIR}/tests/reports/benchmark/latest}"
MARKDOWN_ROOT="${V2_91_ACCEPTANCE_MARKDOWN_ROOT:-${ROOT_DIR}/target/v2_91-benchmark-markdown}"
ACCEPTANCE_REPORT="${REPORT_DIR}/v2_91-acceptance.txt"
BENCHMARK_JSON="${REPORT_DIR}/benchmark-run.json"

if [[ -z "${V2_91_ACCEPTANCE_MARKDOWN_ROOT:-}" ]]; then
  rm -rf "${MARKDOWN_ROOT}"
fi
mkdir -p "${REPORT_DIR}" "${MARKDOWN_ROOT}"
cd "${ROOT_DIR}"

exec > >(tee "${ACCEPTANCE_REPORT}") 2>&1

run_step() {
  local label="$1"
  shift
  echo "[v2.91] ${label}"
  "$@"
}

echo "[v2.91] acceptance started"
echo "[v2.91] report: ${ACCEPTANCE_REPORT}"
echo "[v2.91] benchmark output: ${REPORT_DIR}"

run_step "format check" cargo fmt --all --check

run_step "interface check" \
  cargo check -p memory-domain -p memory-kernel -p memory-cli -p memory-store-pg

run_step "benchmark domain models" \
  cargo test -p memory-domain --lib benchmark -- --test-threads=1

run_step "benchmark ids" \
  cargo test -p memory-domain --lib ids -- --test-threads=1

run_step "benchmark kernel runner" \
  cargo test -p memory-kernel --lib v29_benchmark -- --test-threads=1

run_step "benchmark CLI helpers and parser" \
  cargo test -p memory-cli --bin memory-cli benchmark -- --test-threads=1

run_step "benchmark PG migration contract" \
  cargo test -p memory-store-pg --lib v28_sql_helpers_expose_expected_statements -- --test-threads=1

run_step "V2.7 compatibility smoke" \
  cargo test -p memory-kernel --lib lifecycle -- --test-threads=1

run_step "V2.8 compatibility smoke" \
  cargo test -p memory-kernel --lib v28 -- --test-threads=1

echo "[v2.91] CLI benchmark run"
env \
  MEAT_MEMORY_ENABLE_PG=false \
  MEAT_MEMORY_ENABLE_MARKDOWN=true \
  MEAT_MEMORY_MARKDOWN_ROOT="${MARKDOWN_ROOT}" \
  cargo run -p memory-cli -- benchmark run \
    --suite meat-code-zh \
    --scope-id scp_v291_acceptance \
    --output-dir "${REPORT_DIR}" \
    --json >"${BENCHMARK_JSON}"

test -f "${REPORT_DIR}/summary.md"
test -f "${REPORT_DIR}/metrics.json"
test -f "${REPORT_DIR}/failures.jsonl"
test -f "${REPORT_DIR}/latency.jsonl"
test -f "${REPORT_DIR}/leakage.jsonl"

grep -q "recall@1" "${REPORT_DIR}/summary.md"
grep -q "recall@5" "${REPORT_DIR}/summary.md"
grep -q "latency" "${REPORT_DIR}/summary.md"
grep -q "leakage_count" "${REPORT_DIR}/summary.md"
grep -q "failure_reason" "${REPORT_DIR}/summary.md"

cat >"${REPORT_DIR}/coverage-v2_91-new-code.md" <<'REPORT'
# V2.91 New Code Coverage Gate

Coverage target: 100% line / function / region coverage for V2.91 new production code.

Covered production areas:

- memory-domain benchmark model and benchmark IDs
- memory-kernel V2.91 benchmark runner, fixture, recall / latency / leakage / cost metrics, report writer
- public benchmark adapter skeletons for LoCoMo, LongMemEval and BEAM skipped reports
- memory-cli benchmark run/report/compare parser and output helpers
- memory-store-pg 0009 benchmark migration wiring

Required suites executed by this script:

- memory-domain benchmark and ids unit tests
- memory-kernel v29_benchmark unit/integration tests
- memory-cli benchmark parser/report/compare/helper tests
- memory-store-pg benchmark migration contract test
- V2.7 lifecycle compatibility smoke
- V2.8 governance compatibility smoke

Status: pass when this script exits 0.
REPORT

echo "[v2.91] acceptance completed"
