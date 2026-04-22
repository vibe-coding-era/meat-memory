#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

cd "${ROOT_DIR}"

run_step() {
  local label="$1"
  shift
  echo "[v2.7] ${label}"
  "$@"
}

run_step "checking lifecycle domain and normalizer models" \
  cargo test -p memory-domain --quiet -- --test-threads=1

run_step "checking lifecycle metrics" \
  cargo test -p memory-observability --quiet -- --test-threads=1

run_step "checking lifecycle PG storage and audit migration" \
  cargo test -p memory-store-pg --quiet -- --test-threads=1

run_step "checking Markdown lifecycle roundtrip" \
  cargo test -p memory-store-md --quiet -- --test-threads=1

run_step "checking kernel lifecycle and governance flows" \
  cargo test -p memory-kernel --quiet -- --test-threads=1

run_step "checking HTTP lifecycle APIs" \
  cargo test -p memory-http --quiet -- --test-threads=1

run_step "checking MCP lifecycle tools" \
  cargo test -p memory-mcp --quiet -- --test-threads=1

run_step "checking CLI lifecycle commands" \
  cargo test -p memory-cli --quiet -- --test-threads=1

run_step "checking workspace interface consistency" \
  cargo check --workspace --all-targets

echo "[v2.7] acceptance completed"
