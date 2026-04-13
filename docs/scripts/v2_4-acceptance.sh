#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

cd "${ROOT_DIR}"

run_step() {
  local label="$1"
  shift
  echo "[v2.4] ${label}"
  "$@"
}

run_step "checking domain layer models" \
  cargo test -p memory-domain --quiet

run_step "checking observability v2_4 metrics" \
  cargo test -p memory-observability --quiet

run_step "checking local project document sync engine" \
  cargo test -p memory-sync --quiet

run_step "checking PostgreSQL source/context/document store" \
  cargo test -p memory-store-pg --test pg_store_integration --quiet

run_step "checking kernel source/context/docs services" \
  cargo test -p memory-kernel --lib --quiet

run_step "checking HTTP source/context/docs/metrics APIs" \
  cargo test -p memory-http --lib --quiet

run_step "checking HTTP V2.4 integration flows" \
  cargo test -p memory-http --test http_api_tests --quiet

run_step "checking MCP V2.4 tools" \
  cargo test -p memory-mcp --lib --quiet

run_step "checking MCP V2.4 integration flows" \
  cargo test -p memory-mcp --test mcp_tools_tests --quiet

run_step "checking CLI V2.4 commands" \
  cargo test -p memory-cli --bins --quiet

run_step "checking CLI V2.4 e2e flows" \
  cargo test -p memory-cli --test cli_e2e --quiet

run_step "checking workspace interface consistency" \
  cargo check --workspace --all-targets

echo "[v2.4] acceptance completed"
