#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$ROOT_DIR"

echo "[required-test] cargo test --workspace --quiet -- --test-threads=1"
cargo test --workspace --quiet -- --test-threads=1

echo "[required-test] SECURITY_REPORT_WRITE=0 ./scripts/security-report.sh"
SECURITY_REPORT_WRITE=0 ./scripts/security-report.sh >/dev/null
