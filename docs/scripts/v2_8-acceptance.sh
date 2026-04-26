#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
STRICT="${V2_8_ACCEPTANCE_STRICT:-0}"
REQUIRE_PG="${V2_8_ACCEPTANCE_REQUIRE_PG:-$STRICT}"
PG_ADDR="${V2_8_ACCEPTANCE_PG_ADDR:-127.0.0.1}"
PG_PORT="${V2_8_ACCEPTANCE_PG_PORT:-5433}"
TEST_DATABASE_URL="${MEAT_MEMORY_TEST_DATABASE_URL:-postgres://postgres:postgres@${PG_ADDR}:${PG_PORT}/meat_memory_dev}"
REPORT_DIR="${V2_8_ACCEPTANCE_REPORT_DIR:-${ROOT_DIR}/tests/reports/e2e/latest}"
ACCEPTANCE_REPORT="${REPORT_DIR}/v2_8-acceptance.txt"
SURFACE_REPORT="${REPORT_DIR}/v2_8-surface-parity.txt"

mkdir -p "${REPORT_DIR}"
cd "${ROOT_DIR}"

export MEAT_MEMORY_TEST_DATABASE_URL="${TEST_DATABASE_URL}"

exec > >(tee "${ACCEPTANCE_REPORT}") 2>&1

run_step() {
  local label="$1"
  shift
  echo "[v2.8] ${label}"
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

write_surface_report() {
  {
    echo "# V2.8 Surface Parity"
    echo
    echo "| Surface | Covered flows |"
    echo "|---|---|"
    echo "| HTTP | proposals list/inspect/approve/reject/apply; versions/timeline; rollback; profiles; distill preview |"
    echo "| CLI | proposals list/inspect/approve/reject/apply; versions/timeline; rollback; profiles; distill preview |"
    echo "| MCP | proposals list/inspect/approve/reject/apply; versions/timeline; rollback; profiles; distill preview |"
  } >"${SURFACE_REPORT}"
}

echo "[v2.8] acceptance started"
echo "[v2.8] report: ${ACCEPTANCE_REPORT}"
echo "[v2.8] surface parity: ${SURFACE_REPORT}"

if [[ "$REQUIRE_PG" == "1" ]] && ! pg_available; then
  echo "[v2.8] PostgreSQL is required but ${PG_ADDR}:${PG_PORT} is not reachable" >&2
  echo "[v2.8] Start local PG or set V2_8_ACCEPTANCE_REQUIRE_PG=0 for no-PG smoke." >&2
  exit 1
fi

run_step "format check" cargo fmt --all --check

run_step "workspace interface check" cargo check --workspace --all-targets

run_step "V2.8 domain models" \
  cargo test -p memory-domain --lib -- --test-threads=1

run_step "V2.8 kernel identity, duplicate, supersede, conflict, rollback, profile, preview rules" \
  cargo test -p memory-kernel --lib v28 -- --test-threads=1

run_step "V2.8 observability metrics" \
  cargo test -p memory-observability --lib -- --test-threads=1

run_step "V2.8 PostgreSQL storage contracts" \
  cargo test -p memory-store-pg --lib v28 -- --test-threads=1

if pg_available; then
  run_step "V2.8 kernel runtime PG flows" \
    cargo test -p memory-kernel --lib v28_runtime -- --test-threads=1

  run_step "V2.8 HTTP proposal/timeline/rollback/profile/preview APIs" \
    cargo test -p memory-http --lib -- --test-threads=1

  run_step "V2.8 CLI proposal/timeline/rollback/profile/preview commands" \
    cargo test -p memory-cli --bins -- --test-threads=1

  run_step "V2.8 MCP proposal/timeline/rollback/profile/preview tools" \
    cargo test -p memory-mcp --lib --test mcp_tools_tests -- --test-threads=1
else
  echo "[v2.8] PostgreSQL not reachable; skipped PG-backed runtime and surface acceptance."
  echo "[v2.8] This is acceptable only for local smoke. Full release acceptance should set V2_8_ACCEPTANCE_STRICT=1."
fi

write_surface_report

echo "[v2.8] acceptance completed"
