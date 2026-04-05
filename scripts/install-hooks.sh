#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if ! command -v git >/dev/null 2>&1; then
  echo "[hooks] git is required"
  exit 1
fi

cd "${ROOT_DIR}"
git config core.hooksPath .githooks
chmod +x .githooks/pre-commit .githooks/commit-msg

echo "[hooks] core.hooksPath set to .githooks"
