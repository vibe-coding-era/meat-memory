#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPORT_DIR="${V2_9_GATE_REPORT_DIR:-${ROOT_DIR}/tests/reports/v2_9/latest}"
GATE_REPORT="${REPORT_DIR}/v2_9-production-gate.txt"
STRICT="${V2_9_GATE_STRICT:-0}"
RUN_SECURITY="${V2_9_GATE_SECURITY:-1}"
RUN_ACCEPTANCE_CHILDREN="${V2_9_GATE_RUN_ACCEPTANCE_CHILDREN:-${STRICT}}"

mkdir -p "${REPORT_DIR}"
cd "${ROOT_DIR}"

exec > >(tee "${GATE_REPORT}") 2>&1

run_step() {
  local label="$1"
  shift
  echo "[v2.9-gate] ${label}"
  "$@"
}

echo "[v2.9-gate] production gate started"
echo "[v2.9-gate] report: ${GATE_REPORT}"
echo "[v2.9-gate] strict: ${STRICT}"
echo "[v2.9-gate] acceptance children: ${RUN_ACCEPTANCE_CHILDREN}"

run_step "format check" cargo fmt --all --check

run_step "workspace clippy" \
  cargo clippy --workspace --all-targets -- -D warnings

run_step "workspace tests without PostgreSQL crate" \
  cargo test --workspace --exclude memory-store-pg --quiet -- --test-threads=1

if [[ "${RUN_SECURITY}" == "1" ]]; then
  run_step "security-focused regression" \
    env SECURITY_REPORT_WRITE=0 ./docs/scripts/security-report.sh >/dev/null
else
  echo "[v2.9-gate] security regression skipped by V2_9_GATE_SECURITY=0"
fi

run_step "V2.9 acceptance aggregate" \
  env V2_9_ACCEPTANCE_RUN_CHILDREN="${RUN_ACCEPTANCE_CHILDREN}" \
    ./docs/scripts/v2_9-acceptance.sh

if [[ "${STRICT}" == "1" ]]; then
  echo "[v2.9-gate] strict mode completed with child acceptance scripts enabled."
else
  echo "[v2.9-gate] local mode completed. Release gate should set V2_9_GATE_STRICT=1."
fi

echo "[v2.9-gate] production gate passed"
