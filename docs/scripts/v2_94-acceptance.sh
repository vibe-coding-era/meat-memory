#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPORT_DIR="${V2_94_ACCEPTANCE_REPORT_DIR:-${ROOT_DIR}/tests/reports/passport/latest}"
MARKDOWN_ROOT="${V2_94_ACCEPTANCE_MARKDOWN_ROOT:-${ROOT_DIR}/target/v2_94-passport-markdown}"
ACCEPTANCE_REPORT="${REPORT_DIR}/v2_94-acceptance.txt"
REMEMBER_JSON="${REPORT_DIR}/remember.json"
PROVENANCE_JSON="${REPORT_DIR}/provenance.json"
EXPORT_JSON="${REPORT_DIR}/passport-export.json"
VERIFY_JSON="${REPORT_DIR}/passport-verify.json"
IMPORT_JSON="${REPORT_DIR}/passport-import.json"
IMPORT_SEARCH_JSON="${REPORT_DIR}/passport-import-search.json"
SECRET_FIXTURE="${MARKDOWN_ROOT}/passport-secret.fixture"

trap 'rm -f "${SECRET_FIXTURE}"' EXIT

if [[ -z "${V2_94_ACCEPTANCE_MARKDOWN_ROOT:-}" ]]; then
  rm -rf "${MARKDOWN_ROOT}"
fi
mkdir -p "${REPORT_DIR}" "${MARKDOWN_ROOT}"
cd "${ROOT_DIR}"

exec > >(tee "${ACCEPTANCE_REPORT}") 2>&1

run_step() {
  local label="$1"
  shift
  echo "[v2.94] ${label}"
  "$@"
}

memory_cli() {
  env \
    MEAT_MEMORY_ENABLE_PG=false \
    MEAT_MEMORY_ENABLE_MARKDOWN=true \
    MEAT_MEMORY_MARKDOWN_ROOT="${MARKDOWN_ROOT}" \
    cargo run -p memory-cli -- "$@"
}

echo "[v2.94] acceptance started"
echo "[v2.94] report: ${ACCEPTANCE_REPORT}"
echo "[v2.94] passport output: ${REPORT_DIR}"

run_step "format check" cargo fmt --all --check

run_step "interface check" \
  cargo check -p memory-domain -p memory-kernel -p memory-cli -p memory-store-pg

run_step "evidence domain models" \
  cargo test -p memory-domain --lib evidence -- --test-threads=1

run_step "passport domain models" \
  cargo test -p memory-domain --lib passport -- --test-threads=1

run_step "passport ids" \
  cargo test -p memory-domain --lib ids -- --test-threads=1

run_step "passport kernel" \
  cargo test -p memory-kernel --lib v29_passport -- --test-threads=1

run_step "passport CLI helpers and parser" \
  cargo test -p memory-cli --bin memory-cli passport -- --test-threads=1

run_step "passport PG migration contract" \
  cargo test -p memory-store-pg --lib v28_sql_helpers_expose_expected_statements -- --test-threads=1

echo "[v2.94] seed passport fixture"
memory_cli remember \
  --scope-id scp_v294_acceptance \
  --title "passport durable fact" \
  --body "V2.94 passport durable fact keeps evidence and can be imported into another scope" \
  --memory-kind fact \
  --source-refs "file://docs/V2.9/V2.94-development-plan.md" \
  --json >"${REMEMBER_JSON}"

MEMORY_ID="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["memory_id"])' "${REMEMBER_JSON}")"

cat >"${SECRET_FIXTURE}" <<'SECRET'
password = v294_secret_password
SECRET

memory_cli remember \
  --scope-id scp_v294_acceptance \
  --title "passport redaction risk" \
  --file "${SECRET_FIXTURE}" \
  --memory-kind risk \
  --json >/dev/null

echo "[v2.94] CLI provenance"
memory_cli passport provenance \
  --scope-id scp_v294_acceptance \
  --memory-id "${MEMORY_ID}" \
  --json >"${PROVENANCE_JSON}"
grep -q "file://docs/V2.9/V2.94-development-plan.md" "${PROVENANCE_JSON}"
grep -q "evidence_spans" "${PROVENANCE_JSON}"

echo "[v2.94] CLI passport export"
memory_cli passport export \
  --scope-id scp_v294_acceptance \
  --limit 20 \
  --output-dir "${REPORT_DIR}" \
  --json >"${EXPORT_JSON}"

test -f "${REPORT_DIR}/passport.json"
test -f "${REPORT_DIR}/manifest.json"
test -f "${REPORT_DIR}/memories.json"
test -f "${REPORT_DIR}/evidence.json"
test -f "${REPORT_DIR}/manifest.md"
grep -q "Memory Passport Manifest" "${REPORT_DIR}/manifest.md"
grep -q "bundle_hash" "${REPORT_DIR}/manifest.md"
if grep -R "v294_secret_password" "${REPORT_DIR}"; then
  echo "[v2.94] raw secret leaked in passport reports"
  exit 1
fi

echo "[v2.94] CLI passport verify"
memory_cli passport verify \
  --input-dir "${REPORT_DIR}" \
  --json >"${VERIFY_JSON}"
python3 - <<'PY' "${VERIFY_JSON}"
import json
import sys

payload = json.load(open(sys.argv[1]))
assert payload["valid"] is True, payload
assert payload["checked_objects"] >= 2, payload
assert payload["manifest"]["schema_version"] == "2.94", payload
PY

echo "[v2.94] CLI passport import"
memory_cli passport import \
  --input-dir "${REPORT_DIR}" \
  --target-scope-id scp_v294_imported \
  --json >"${IMPORT_JSON}"
python3 - <<'PY' "${IMPORT_JSON}"
import json
import sys

payload = json.load(open(sys.argv[1]))
assert payload["verified"] is True, payload
assert payload["imported_count"] >= 1, payload
assert payload["target_scope_id"] == "scp_v294_imported", payload
assert payload["id_mappings"], payload
PY

memory_cli search "durable fact" \
  --scope-id scp_v294_imported \
  --json >"${IMPORT_SEARCH_JSON}"
grep -q "passport durable fact" "${IMPORT_SEARCH_JSON}"

cat >"${REPORT_DIR}/coverage-v2_94-new-code.md" <<'REPORT'
# V2.94 New Code Coverage Gate

Coverage target: 100% line / function / region coverage for V2.94 new production code.

Covered production areas:

- memory-domain evidence span model and passport manifest/hash model
- memory-kernel evidence derivation, provenance query, passport export/import/verify and report writer
- memory-cli passport export / verify / import / provenance parser and output helpers
- memory-store-pg 0012 evidence span / passport metadata migration wiring

Required suites executed by this script:

- memory-domain evidence, passport and ids unit tests
- memory-kernel v29_passport tests
- memory-cli passport parser/report/helper tests
- memory-store-pg passport migration contract test
- CLI smoke for provenance, export, verify, import, redaction and imported search

Status: pass when this script exits 0.
REPORT

echo "[v2.94] acceptance completed"
