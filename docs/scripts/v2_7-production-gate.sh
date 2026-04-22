#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
STRICT="${V2_7_GATE_STRICT:-0}"
REQUIRE_PG="${V2_7_GATE_REQUIRE_PG:-$STRICT}"
RUN_COVERAGE="${V2_7_GATE_COVERAGE:-0}"
PG_ADDR="${V2_7_GATE_PG_ADDR:-127.0.0.1}"
PG_PORT="${V2_7_GATE_PG_PORT:-5433}"
COVERAGE_REPORT="${V2_7_GATE_COVERAGE_REPORT:-}"
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
  echo "[v2.7-gate] ${label}"
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
  echo "[v2.7-gate] PostgreSQL is required but ${PG_ADDR}:${PG_PORT} is not reachable" >&2
  echo "[v2.7-gate] Start it with ./docs/scripts/dev-db-up.sh or set V2_7_GATE_REQUIRE_PG=0 for local no-PG smoke." >&2
  exit 1
fi

if [[ "$RUN_COVERAGE" == "1" ]]; then
  if [[ -z "$COVERAGE_REPORT" ]]; then
    echo "[v2.7-gate] V2_7_GATE_COVERAGE_REPORT is required when V2_7_GATE_COVERAGE=1" >&2
    echo "[v2.7-gate] The report must include all required rows checked by v2_7-coverage-threshold.py." >&2
    exit 1
  fi
  if [[ ! -f "$COVERAGE_REPORT" ]]; then
    echo "[v2.7-gate] Coverage report not found: $COVERAGE_REPORT" >&2
    echo "[v2.7-gate] Generate focused coverage first, then rerun with V2_7_GATE_COVERAGE_REPORT=<path>." >&2
    exit 1
  fi
fi

run_step "format check" cargo fmt --all --check

run_step "workspace interface check" cargo check --workspace --all-targets

run_step "P1 lifecycle regression suite" \
  cargo test -p memory-kernel --lib lifecycle -- --test-threads=1

run_step "kernel integration flows" \
  cargo test -p memory-kernel --test kernel_flow_tests -- --test-threads=1

run_step "domain, observability, markdown, pg storage unit suites" \
  cargo test -p memory-domain -p memory-observability -p memory-store-md -p memory-store-pg --lib -- --test-threads=1

run_step "HTTP, MCP, CLI entrypoint suites" \
  cargo test -p memory-http -p memory-mcp -p memory-cli -- --test-threads=1

run_step "security-focused regression" \
  env SECURITY_REPORT_WRITE=0 ./docs/scripts/security-report.sh >/dev/null

if pg_available; then
  run_step "full V2.7 acceptance with PostgreSQL available" \
    ./docs/scripts/v2_7-acceptance.sh
else
  echo "[v2.7-gate] PostgreSQL not reachable; skipped full V2.7 acceptance."
  echo "[v2.7-gate] This is acceptable only for local no-PG smoke. Production/release gate must set V2_7_GATE_STRICT=1."
fi

if [[ "$RUN_COVERAGE" == "1" ]]; then
  run_step "coverage threshold check" \
    python3 ./docs/scripts/v2_7-coverage-threshold.py "$COVERAGE_REPORT"
fi

echo "[v2.7-gate] production gate completed"
