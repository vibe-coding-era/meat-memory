#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "[bootstrap] project root: ${ROOT_DIR}"

if ! command -v rustup >/dev/null 2>&1; then
  echo "[bootstrap] rustup is required but not installed"
  exit 1
fi

echo "[bootstrap] ensuring Rust components"
rustup component add rustfmt clippy

echo "[bootstrap] cargo metadata"
(cd "${ROOT_DIR}" && cargo metadata --format-version 1 >/dev/null)

if [ -d "${ROOT_DIR}/.git" ] && [ -f "${ROOT_DIR}/scripts/install-hooks.sh" ]; then
  echo "[bootstrap] installing git hooks"
  "${ROOT_DIR}/scripts/install-hooks.sh" >/dev/null
fi

echo "[bootstrap] workspace ready"
