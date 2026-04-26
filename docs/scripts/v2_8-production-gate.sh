#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
STRICT="${V2_8_GATE_STRICT:-0}"
REQUIRE_PG="${V2_8_GATE_REQUIRE_PG:-$STRICT}"
RUN_SECURITY="${V2_8_GATE_SECURITY:-1}"
PG_ADDR="${V2_8_GATE_PG_ADDR:-127.0.0.1}"
PG_PORT="${V2_8_GATE_PG_PORT:-5433}"
TEST_DATABASE_URL="${MEAT_MEMORY_TEST_DATABASE_URL:-postgres://postgres:postgres@${PG_ADDR}:${PG_PORT}/meat_memory_dev}"
DATABASE_URL="${MEAT_MEMORY_DATABASE_URL:-${TEST_DATABASE_URL}}"
PGVECTOR_URL="${MEAT_MEMORY_PGVECTOR_URL:-${TEST_DATABASE_URL}}"

cd "${ROOT_DIR}"

export MEAT_MEMORY_TEST_DATABASE_URL="${TEST_DATABASE_URL}"
export MEAT_MEMORY_DATABASE_URL="${DATABASE_URL}"
export MEAT_MEMORY_PGVECTOR_URL="${PGVECTOR_URL}"

run_step() {
  local label="$1"
  shift
  echo "[v2.8-gate] ${label}"
  "$@"
}

pg_available() {
  python3 - "$PG_ADDR" "$PG_PORT" <<'PY'
import socket
import sys

host = sys.argv[1]
port = int(sys.argv[2])
sock = socket.socket()
sock.settimeout(0.25)
try:
    sock.connect((host, port))
except OSError:
    sys.exit(1)
finally:
    sock.close()
PY
}

if [[ "$REQUIRE_PG" == "1" ]] && ! pg_available; then
  echo "[v2.8-gate] PostgreSQL is required but ${PG_ADDR}:${PG_PORT} is not reachable" >&2
  echo "[v2.8-gate] Start it with ./docs/scripts/dev-db-up.sh or set V2_8_GATE_REQUIRE_PG=0 for local no-PG smoke." >&2
  exit 1
fi

run_step "format check" cargo fmt --all --check

run_step "workspace interface check" cargo check --workspace --all-targets

run_step "V2.8 domain models" \
  cargo test -p memory-domain --lib -- --test-threads=1

run_step "V2.8 PostgreSQL storage contracts" \
  cargo test -p memory-store-pg --lib v28 -- --test-threads=1

run_step "V2.8 kernel governance, rollback, profile, preview rules" \
  cargo test -p memory-kernel --lib v28 -- --test-threads=1

run_step "V2.8 metrics" \
  cargo test -p memory-observability --lib -- --test-threads=1

run_step "V2.8 HTTP, MCP, CLI surface suites" \
  cargo test -p memory-http -p memory-mcp -p memory-cli -- --test-threads=1

if [[ "$RUN_SECURITY" == "1" ]]; then
  run_step "security-focused regression" \
    env SECURITY_REPORT_WRITE=0 ./docs/scripts/security-report.sh >/dev/null
fi

if pg_available; then
  run_step "strict V2.8 acceptance with PostgreSQL available" \
    env V2_8_ACCEPTANCE_STRICT=1 ./docs/scripts/v2_8-acceptance.sh
else
  echo "[v2.8-gate] PostgreSQL not reachable; skipped strict V2.8 acceptance."
  echo "[v2.8-gate] This is acceptable only for local no-PG smoke. Release gate should set V2_8_GATE_STRICT=1."
fi

echo "[v2.8-gate] production gate completed"
