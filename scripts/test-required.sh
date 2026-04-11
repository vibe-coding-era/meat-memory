#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$ROOT_DIR"

echo "[required-test] cargo test --workspace --quiet"
cargo test --workspace --quiet

echo "[required-test] ./scripts/security-report.sh"
./scripts/security-report.sh >/dev/null
