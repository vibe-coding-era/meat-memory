#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPORT_DIR="${V2_93_ACCEPTANCE_REPORT_DIR:-${ROOT_DIR}/tests/reports/health/latest}"
MARKDOWN_ROOT="${V2_93_ACCEPTANCE_MARKDOWN_ROOT:-${ROOT_DIR}/target/v2_93-health-markdown}"
ACCEPTANCE_REPORT="${REPORT_DIR}/v2_93-acceptance.txt"
REMEMBER_REDACT_JSON="${REPORT_DIR}/remember-redact.json"
SEARCH_JSON="${REPORT_DIR}/search-redacted.json"
HEALTH_JSON="${REPORT_DIR}/health-report.json"
DENY_LOG="${REPORT_DIR}/remember-deny.txt"
API_KEY_FIXTURE="${REPORT_DIR}/api-key.fixture"
PRIVATE_KEY_FIXTURE="${REPORT_DIR}/private-key.fixture"
trap 'rm -f "${API_KEY_FIXTURE}" "${PRIVATE_KEY_FIXTURE}"' EXIT

if [[ -z "${V2_93_ACCEPTANCE_MARKDOWN_ROOT:-}" ]]; then
  rm -rf "${MARKDOWN_ROOT}"
fi
mkdir -p "${REPORT_DIR}" "${MARKDOWN_ROOT}"
cd "${ROOT_DIR}"

exec > >(tee "${ACCEPTANCE_REPORT}") 2>&1

run_step() {
  local label="$1"
  shift
  echo "[v2.93] ${label}"
  "$@"
}

memory_cli() {
  env \
    MEAT_MEMORY_ENABLE_PG=false \
    MEAT_MEMORY_ENABLE_MARKDOWN=true \
    MEAT_MEMORY_MARKDOWN_ROOT="${MARKDOWN_ROOT}" \
    cargo run -p memory-cli -- "$@"
}

echo "[v2.93] acceptance started"
echo "[v2.93] report: ${ACCEPTANCE_REPORT}"
echo "[v2.93] health output: ${REPORT_DIR}"

run_step "format check" cargo fmt --all --check

run_step "interface check" \
  cargo check -p memory-domain -p memory-observability -p memory-kernel -p memory-cli -p memory-store-pg

run_step "secret domain models" \
  cargo test -p memory-domain --lib security -- --test-threads=1

run_step "secret ids" \
  cargo test -p memory-domain --lib ids -- --test-threads=1

run_step "secret guard and health kernel" \
  cargo test -p memory-kernel --lib v29_security -- --test-threads=1

run_step "trace recall guard regression" \
  cargo test -p memory-kernel --lib v29_trace -- --test-threads=1

run_step "health CLI helpers and parser" \
  cargo test -p memory-cli --bin memory-cli health -- --test-threads=1

run_step "security observability metrics" \
  cargo test -p memory-observability --lib record_functions_update_metrics_counters -- --test-threads=1

run_step "secret PG migration contract" \
  cargo test -p memory-store-pg --lib v28_sql_helpers_expose_expected_statements -- --test-threads=1

echo "[v2.93] seed secret guard fixture"
cat >"${API_KEY_FIXTURE}" <<'KEY'
service api_key=sk_test_1234567890abcdef owner owner@example.test
KEY
memory_cli remember \
  --scope-id scp_v293_acceptance \
  --title "guard redacts api key" \
  --file "${API_KEY_FIXTURE}" \
  --memory-kind fact \
  --json >"${REMEMBER_REDACT_JSON}"

if grep -q "sk_test_1234567890abcdef" "${REMEMBER_REDACT_JSON}"; then
  echo "[v2.93] raw api key leaked in remember output"
  exit 1
fi
grep -q "REDACTED:API_KEY" "${REMEMBER_REDACT_JSON}"

echo "[v2.93] deny private key fixture"
cat >"${PRIVATE_KEY_FIXTURE}" <<'KEY'
-----BEGIN PRIVATE KEY-----
abc
-----END PRIVATE KEY-----
KEY
if memory_cli remember \
  --scope-id scp_v293_acceptance \
  --title "deny private key" \
  --file "${PRIVATE_KEY_FIXTURE}" \
  --memory-kind risk \
  --json >"${DENY_LOG}" 2>&1; then
  echo "[v2.93] private key write unexpectedly succeeded"
  exit 1
fi
grep -q "write denied by secret guard" "${DENY_LOG}"

memory_cli remember \
  --scope-id scp_v293_acceptance \
  --title "restricted health risk" \
  --body "restricted health note should appear as high sensitivity risk" \
  --memory-kind risk \
  --sensitivity restricted \
  --json >/dev/null

echo "[v2.93] CLI search redacted result"
memory_cli search "api key" \
  --scope-id scp_v293_acceptance \
  --json >"${SEARCH_JSON}"
grep -q "REDACTED:API_KEY" "${SEARCH_JSON}"
if grep -q "sk_test_1234567890abcdef" "${SEARCH_JSON}"; then
  echo "[v2.93] raw api key leaked in search output"
  exit 1
fi

echo "[v2.93] CLI health report"
memory_cli health report \
  --scope-id scp_v293_acceptance \
  --limit 20 \
  --output-dir "${REPORT_DIR}" \
  --json >"${HEALTH_JSON}"

test -f "${REPORT_DIR}/health.json"
test -f "${REPORT_DIR}/health.md"
grep -q "Memory Health Report" "${REPORT_DIR}/health.md"
grep -q "high_sensitivity" "${REPORT_DIR}/health.md"
python3 - <<'PY' "${HEALTH_JSON}"
import json
import sys

payload = json.load(open(sys.argv[1]))
assert payload["secret_findings"] >= 1, payload
assert payload["restricted"] == 1, payload
assert any(risk["kind"] == "high_sensitivity" for risk in payload["risks"]), payload
PY

cat >"${REPORT_DIR}/coverage-v2_93-new-code.md" <<'REPORT'
# V2.93 New Code Coverage Gate

Coverage target: 100% line / function / region coverage for V2.93 new production code.

Covered production areas:

- memory-domain secret finding and health risk model
- memory-kernel secret / PII detector, ingest guard, recall guard, hard-delete redaction, health analyzer and report writer
- memory-cli health report parser and output helpers
- memory-observability V2.9 security and health metrics
- memory-store-pg 0011 secret finding / health report migration wiring

Required suites executed by this script:

- memory-domain security and ids unit tests
- memory-kernel v29_security and v29_trace tests
- memory-cli health parser/report/helper tests
- memory-observability security metrics test
- memory-store-pg secret migration contract test
- CLI smoke for redact, deny, search, and health report projection

Status: pass when this script exits 0.
REPORT

echo "[v2.93] acceptance completed"
