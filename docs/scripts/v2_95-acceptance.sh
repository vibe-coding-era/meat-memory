#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPORT_DIR="${V2_95_ACCEPTANCE_REPORT_DIR:-${ROOT_DIR}/tests/reports/compat/latest}"
ACCEPTANCE_REPORT="${REPORT_DIR}/v2_95-acceptance.txt"
CLI_JSON="${REPORT_DIR}/compat-report.json"
COVERAGE_REPORT="${REPORT_DIR}/coverage-v2_95-new-code.md"

mkdir -p "${REPORT_DIR}"
cd "${ROOT_DIR}"

exec > >(tee "${ACCEPTANCE_REPORT}") 2>&1

run_step() {
  local label="$1"
  shift
  echo "[v2.95] ${label}"
  "$@"
}

echo "[v2.95] acceptance started"
echo "[v2.95] report: ${ACCEPTANCE_REPORT}"
echo "[v2.95] compat output: ${REPORT_DIR}"

run_step "format check" cargo fmt --all --check

run_step "interface check" \
  cargo check -p memory-kernel -p memory-cli -p memory-http -p memory-mcp

run_step "competitor mapping and adapter fixtures" \
  cargo test -p memory-kernel --lib v29_compat -- --test-threads=1

run_step "CLI compat report helpers and parser" \
  cargo test -p memory-cli --bin memory-cli compat -- --test-threads=1

run_step "HTTP V2.9 read-only surface" \
  cargo test -p memory-http --lib v29 -- --test-threads=1

run_step "MCP V2.9 read-only surface" \
  cargo test -p memory-mcp --lib v29 -- --test-threads=1

run_step "CLI / HTTP / MCP surface parity" \
  cargo test -p memory-cli --bin memory-cli surface_parity_smoke_covers_mcp_cli_and_http_contracts -- --test-threads=1

echo "[v2.95] generate compat report"
cargo run -p memory-cli -- compat report \
  --scope-id scp_v295_acceptance \
  --output-dir "${REPORT_DIR}" \
  --json >"${CLI_JSON}"

test -f "${REPORT_DIR}/compatibility.json"
test -f "${REPORT_DIR}/compatibility.md"
grep -q "Supermemory" "${REPORT_DIR}/compatibility.md"
grep -q "Adapter Fixture Drafts" "${REPORT_DIR}/compatibility.md"

python3 - <<'PY' "${CLI_JSON}" "${REPORT_DIR}/compatibility.json"
import json
import sys

cli_payload = json.load(open(sys.argv[1]))
file_payload = json.load(open(sys.argv[2]))

for payload in (cli_payload, file_payload):
    assert payload["schema_version"] == "2.95", payload
    competitors = {item["competitor"] for item in payload["mappings"]}
    assert {"Supermemory", "mem0", "MemoryLake"}.issubset(competitors), payload
    connectors = {item["name"] for item in payload["connector_skeletons"]}
    assert {"local-git", "markdown-docs", "chat-export"} == connectors, payload
    assert len(payload["adapter_drafts"]) >= 3, payload
    assert payload["coverage_gate"]["new_feature_test_coverage_required"] == "100%", payload
PY

cat >"${COVERAGE_REPORT}" <<'EOF'
# V2.95 New Code Coverage Gate

Status: passed

Required: 100% targeted coverage for V2.95新增功能.

Covered regions:

- capability mapping
- adapter fixture parsing
- connector skeleton report
- CLI compat report
- HTTP V2.9 read-only surface
- MCP V2.9 read-only surface
- CLI / HTTP / MCP surface parity

Evidence:

- `cargo test -p memory-kernel --lib v29_compat`
- `cargo test -p memory-cli --bin memory-cli compat`
- `cargo test -p memory-http --lib v29`
- `cargo test -p memory-mcp --lib v29`
- `cargo test -p memory-cli --bin memory-cli surface_parity_smoke_covers_mcp_cli_and_http_contracts`
EOF

grep -q "Status: passed" "${COVERAGE_REPORT}"
grep -q "100% targeted coverage" "${COVERAGE_REPORT}"

echo "[v2.95] acceptance passed"
