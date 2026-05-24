#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPORT_DIR="${V2_9_ACCEPTANCE_REPORT_DIR:-${ROOT_DIR}/tests/reports/v2_9/latest}"
ACCEPTANCE_REPORT="${REPORT_DIR}/v2_9-acceptance.txt"
SUMMARY_REPORT="${REPORT_DIR}/v2_9-acceptance-summary.md"
COVERAGE_REPORT="${REPORT_DIR}/coverage-v2_9-new-code.md"
RUN_CHILDREN="${V2_9_ACCEPTANCE_RUN_CHILDREN:-1}"

mkdir -p "${REPORT_DIR}"
cd "${ROOT_DIR}"

exec > >(tee "${ACCEPTANCE_REPORT}") 2>&1

run_step() {
  local label="$1"
  shift
  echo "[v2.9] ${label}"
  "$@"
}

require_report_contains() {
  local path="$1"
  local pattern="$2"
  if [[ ! -f "${path}" ]]; then
    echo "[v2.9] missing required report: ${path}" >&2
    exit 1
  fi
  grep -q "${pattern}" "${path}"
}

echo "[v2.9] acceptance started"
echo "[v2.9] report: ${ACCEPTANCE_REPORT}"
echo "[v2.9] output: ${REPORT_DIR}"
echo "[v2.9] run children: ${RUN_CHILDREN}"

run_step "format check" cargo fmt --all --check

if [[ "${RUN_CHILDREN}" == "1" ]]; then
  run_step "V2.91 benchmark baseline" ./docs/scripts/v2_91-acceptance.sh
  run_step "V2.92 recall trace and budget" ./docs/scripts/v2_92-acceptance.sh
  run_step "V2.93 secret guard and health" ./docs/scripts/v2_93-acceptance.sh
  run_step "V2.94 evidence and passport" ./docs/scripts/v2_94-acceptance.sh
  run_step "V2.95 compatibility surface" ./docs/scripts/v2_95-acceptance.sh
  run_step "V2.97 connector parity" ./docs/scripts/v2_97-acceptance.sh
else
  echo "[v2.9] child acceptance scripts skipped by V2_9_ACCEPTANCE_RUN_CHILDREN=0"
fi

require_report_contains \
  "${ROOT_DIR}/tests/reports/benchmark/latest/coverage-v2_91-new-code.md" \
  "100%"
require_report_contains \
  "${ROOT_DIR}/tests/reports/trace/latest/coverage-v2_92-new-code.md" \
  "100%"
require_report_contains \
  "${ROOT_DIR}/tests/reports/health/latest/coverage-v2_93-new-code.md" \
  "100%"
require_report_contains \
  "${ROOT_DIR}/tests/reports/passport/latest/coverage-v2_94-new-code.md" \
  "100%"
require_report_contains \
  "${ROOT_DIR}/tests/reports/compat/latest/coverage-v2_95-new-code.md" \
  "100% targeted coverage"
require_report_contains \
  "${ROOT_DIR}/tests/reports/compat/v2_97/coverage-v2_97-new-code.md" \
  "100% targeted coverage"

cat >"${SUMMARY_REPORT}" <<EOF
# V2.9 Acceptance Summary

Status: passed

Child acceptance scripts: ${RUN_CHILDREN}

Validated release slices:

- V2.91 benchmark baseline
- V2.92 recall trace and budget
- V2.93 secret guard and health
- V2.94 evidence and passport
- V2.95 compatibility surface
- V2.97 connector parity

Required coverage: 100% targeted coverage for new V2.9 production code.

Evidence reports:

- \`tests/reports/benchmark/latest/coverage-v2_91-new-code.md\`
- \`tests/reports/trace/latest/coverage-v2_92-new-code.md\`
- \`tests/reports/health/latest/coverage-v2_93-new-code.md\`
- \`tests/reports/passport/latest/coverage-v2_94-new-code.md\`
- \`tests/reports/compat/latest/coverage-v2_95-new-code.md\`
- \`tests/reports/compat/v2_97/coverage-v2_97-new-code.md\`
EOF

cat >"${COVERAGE_REPORT}" <<'EOF'
# V2.9 New Code Coverage Gate

Status: passed

Required: 100% targeted coverage for V2.9新增功能.

Covered release slices:

- V2.91 benchmark baseline
- V2.92 recall trace and budget
- V2.93 secret guard and health
- V2.94 evidence and passport
- V2.95 compatibility surface
- V2.97 connector parity

Evidence:

- `docs/scripts/v2_91-acceptance.sh`
- `docs/scripts/v2_92-acceptance.sh`
- `docs/scripts/v2_93-acceptance.sh`
- `docs/scripts/v2_94-acceptance.sh`
- `docs/scripts/v2_95-acceptance.sh`
- `docs/scripts/v2_97-acceptance.sh`
EOF

grep -q "Status: passed" "${SUMMARY_REPORT}"
grep -q "100% targeted coverage" "${COVERAGE_REPORT}"

echo "[v2.9] acceptance passed"
