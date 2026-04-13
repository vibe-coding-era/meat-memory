#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPORTS_DIR="$ROOT_DIR/tests/reports"
TIMESTAMP="$(date '+%Y%m%d-%H%M%S')"

prepare_category() {
  local category="$1"
  mkdir -p "$REPORTS_DIR/$category/latest" "$REPORTS_DIR/$category/archive"
  rm -rf "$REPORTS_DIR/$category/latest"/*
}

run_command_report() {
  local category="$1"
  local name="$2"
  shift 2
  local latest_file="$REPORTS_DIR/$category/latest/$name"
  local archive_file="$REPORTS_DIR/$category/archive/${TIMESTAMP}-$name"

  (
    echo "# report_category: $category"
    echo "# report_name: $name"
    echo "# started_at: $(date '+%Y-%m-%d %H:%M:%S %Z')"
    echo "# workdir: $ROOT_DIR"
    echo "# command: $*"
    echo
    "$@"
  ) 2>&1 | tee "$latest_file"
  local status=${PIPESTATUS[0]}

  {
    echo
    echo "# exit_code: $status"
    echo "# finished_at: $(date '+%Y-%m-%d %H:%M:%S %Z')"
  } | tee -a "$latest_file" >/dev/null

  cp "$latest_file" "$archive_file"
  return "$status"
}

run_multiline_report() {
  local category="$1"
  local name="$2"
  shift 2
  local latest_file="$REPORTS_DIR/$category/latest/$name"
  local archive_file="$REPORTS_DIR/$category/archive/${TIMESTAMP}-$name"
  local -a commands=("$@")

  (
    echo "# report_category: $category"
    echo "# report_name: $name"
    echo "# started_at: $(date '+%Y-%m-%d %H:%M:%S %Z')"
    echo "# workdir: $ROOT_DIR"
    echo
    for cmd in "${commands[@]}"; do
      echo "## command: $cmd"
      eval "$cmd"
      echo
    done
  ) 2>&1 | tee "$latest_file"
  local status=${PIPESTATUS[0]}

  {
    echo "# exit_code: $status"
    echo "# finished_at: $(date '+%Y-%m-%d %H:%M:%S %Z')"
  } | tee -a "$latest_file" >/dev/null

  cp "$latest_file" "$archive_file"
  return "$status"
}

write_index() {
  cat > "$REPORTS_DIR/latest-run.md" <<EOF
# Latest Test Report Run

- generated_at: $(date '+%Y-%m-%d %H:%M:%S %Z')
- archive_tag: $TIMESTAMP
- unit_report: \`tests/reports/unit/latest/workspace-unit.txt\`
- integration_report: \`tests/reports/integration/latest/v2-integration.txt\`
- e2e_report: \`tests/reports/e2e/latest/cli-and-acceptance.txt\`
- v21_report: \`tests/reports/e2e/latest/v2_1-acceptance.txt\`
- v24_report: \`tests/reports/e2e/latest/v2_4-acceptance.txt\`
- perf_summary: \`tests/reports/perf/latest/perf-summary.txt\`
- perf_kernel: \`tests/reports/perf/latest/kernel-perf.txt\`
- perf_sync: \`tests/reports/perf/latest/sync-perf.txt\`
EOF
}

prepare_category "unit"
prepare_category "integration"
prepare_category "e2e"
prepare_category "perf"
prepare_category "security"

rm -rf "$ROOT_DIR/target/perf"

run_command_report "unit" "workspace-unit.txt" \
  cargo test --workspace --lib --bins --quiet -- --skip lazy_pool_operations_surface_errors_after_building_queries

run_multiline_report "integration" "v2-integration.txt" \
  "cargo test -p memory-kernel --test kernel_flow_tests --quiet -- --test-threads=1" \
  "cargo test -p memory-http --test http_api_tests --quiet -- --test-threads=1" \
  "cargo test -p memory-mcp --test mcp_tools_tests --quiet -- --test-threads=1" \
  "cargo test -p memory-sync --test sync_merge_tests --quiet -- --test-threads=1"

run_multiline_report "e2e" "cli-and-acceptance.txt" \
  "cargo test -p memory-cli --test cli_e2e --quiet" \
  "./docs/scripts/v1-acceptance.sh"

run_command_report "e2e" "v2_1-acceptance.txt" \
  ./docs/scripts/v2_1-acceptance.sh

run_command_report "e2e" "v2_4-acceptance.txt" \
  ./docs/scripts/v2_4-acceptance.sh

run_command_report "perf" "perf-summary.txt" \
  ./docs/scripts/v2-perf.sh "$REPORTS_DIR/perf/latest"
cp "$REPORTS_DIR/perf/latest/kernel-perf.txt" \
  "$REPORTS_DIR/perf/archive/${TIMESTAMP}-kernel-perf.txt"
cp "$REPORTS_DIR/perf/latest/sync-perf.txt" \
  "$REPORTS_DIR/perf/archive/${TIMESTAMP}-sync-perf.txt"

run_command_report "security" "security-summary.txt" \
  ./docs/scripts/security-report.sh "$REPORTS_DIR/security/latest"

write_index
rm -rf "$ROOT_DIR/target/perf"
