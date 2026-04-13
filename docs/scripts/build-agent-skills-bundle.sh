#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TARGET="${1:-all}"
DIST_DIR="${2:-$ROOT_DIR/dist/release}"
EXPORT_DIR="${DIST_DIR}/agent-skills-bundle"
ZIP_PATH="${DIST_DIR}/agent-skills-bundle.zip"

mkdir -p "${DIST_DIR}"
rm -rf "${EXPORT_DIR}" "${ZIP_PATH}"

"${ROOT_DIR}/docs/scripts/export-agent-skills.sh" "${TARGET}" "${EXPORT_DIR}"

(
  cd "${DIST_DIR}"
  zip -qr "$(basename "${ZIP_PATH}")" "$(basename "${EXPORT_DIR}")"
)

echo "Built ${ZIP_PATH}"
