#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPORT_DIR="${1:-$ROOT_DIR/tests/reports/security/latest}"
ARCHIVE_DIR="$ROOT_DIR/tests/reports/security/archive"
TIMESTAMP="$(date '+%Y%m%d-%H%M%S')"
SUMMARY_FILE="$REPORT_DIR/security-summary.txt"
ARCHIVE_FILE="$ARCHIVE_DIR/${TIMESTAMP}-security-summary.txt"

mkdir -p "$REPORT_DIR" "$ARCHIVE_DIR"

run_test() {
  local title="$1"
  shift
  echo "## $title"
  echo "\$ $*"
  "$@"
  echo
}

{
  echo "# Security Summary"
  echo
  echo "- generated_at: $(date '+%Y-%m-%d %H:%M:%S %Z')"
  echo "- workdir: $ROOT_DIR"
  echo "- focus: authorization, hijack prevention, injection boundaries, payload limits"
  echo
  run_test "HTTP security-focused regression" \
    cargo test -p memory-http --test http_api_tests --quiet -- --test-threads=1
  run_test "MCP security-sensitive regression" \
    cargo test -p memory-mcp --test mcp_tools_tests --quiet -- --test-threads=1
} > "$SUMMARY_FILE" 2>&1

cp "$SUMMARY_FILE" "$ARCHIVE_FILE"

cat > "$REPORT_DIR/README.md" <<EOF
# Security Report

- generated_at: $(date '+%Y-%m-%d %H:%M:%S %Z')
- summary: \`tests/reports/security/latest/security-summary.txt\`
- archive_copy: \`tests/reports/security/archive/${TIMESTAMP}-security-summary.txt\`
- coverage: HTTP sensitive routes, MCP publish/promote auth, payload boundary regression
EOF

cp "$REPORT_DIR/README.md" "$ARCHIVE_DIR/${TIMESTAMP}-README.md"
cat "$SUMMARY_FILE"
