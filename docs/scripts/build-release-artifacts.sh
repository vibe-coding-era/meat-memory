#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TARGET=""
ASSET_NAME=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --target)
      TARGET="$2"
      shift 2
      ;;
    --asset-name)
      ASSET_NAME="$2"
      shift 2
      ;;
    *)
      echo "Unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

if [[ -z "${TARGET}" || -z "${ASSET_NAME}" ]]; then
  echo "Usage: $0 --target <rust-target> --asset-name <archive-prefix>" >&2
  exit 1
fi

BIN_DIR="${ROOT_DIR}/target/${TARGET}/release"
DIST_DIR="${ROOT_DIR}/dist/release"
STAGE_DIR="${DIST_DIR}/${ASSET_NAME}"
ARCHIVE_PATH="${DIST_DIR}/${ASSET_NAME}.tar.gz"
README_PATH="${STAGE_DIR}/README.txt"
CONFIG_PATH="${STAGE_DIR}/config/app.toml"

mkdir -p "${DIST_DIR}"
rm -rf "${STAGE_DIR}"
mkdir -p "${STAGE_DIR}/bin" "${STAGE_DIR}/config"

for binary in memory-cli memory-app memory-worker; do
  if [[ ! -f "${BIN_DIR}/${binary}" ]]; then
    echo "Missing built binary: ${BIN_DIR}/${binary}" >&2
    exit 1
  fi
  cp "${BIN_DIR}/${binary}" "${STAGE_DIR}/bin/${binary}"
done

cp "${ROOT_DIR}/config/app.toml" "${CONFIG_PATH}"
awk '
  /^\[[^]]+\]$/ { section = $0 }
  section == "[markdown]" && $0 ~ /^root[[:space:]]*=/ {
    print "root = \"./data/markdown\""
    next
  }
  section == "[assets]" && $0 ~ /^root[[:space:]]*=/ {
    print "root = \"./data/assets\""
    next
  }
  section == "[access]" && $0 ~ /^key_store_path[[:space:]]*=/ {
    print "key_store_path = \"./data/keys/default-key.toml\""
    next
  }
  section == "[sync]" && $0 ~ /^mode[[:space:]]*=/ {
    print "mode = \"local_only\""
    if (!sync_state_written) {
      print "state_path = \"./data/sync/state.json\""
      sync_state_written = 1
    }
    next
  }
  section == "[sync]" && $0 ~ /^state_path[[:space:]]*=/ {
    if (!sync_state_written) {
      print "state_path = \"./data/sync/state.json\""
      sync_state_written = 1
    }
    next
  }
  section == "[features]" && $0 ~ /^enable_pg[[:space:]]*=/ {
    print "enable_pg = false"
    next
  }
  section == "[features]" && $0 ~ /^enable_markdown[[:space:]]*=/ {
    print "enable_markdown = true"
    next
  }
  { print }
' "${CONFIG_PATH}" > "${CONFIG_PATH}.tmp"
mv "${CONFIG_PATH}.tmp" "${CONFIG_PATH}"

cat > "${README_PATH}" <<EOF
Meat Memory release bundle
==========================

Archive: ${ASSET_NAME}.tar.gz
Rust target: ${TARGET}

Contents
- bin/memory-cli
- bin/memory-app
- bin/memory-worker
- config/app.toml

Quick start
1. config/app.toml defaults to markdown-only local storage under ./data.
2. Run MEAT_MEMORY_CONFIG=./config/app.toml ./bin/memory-cli --help
3. Verify a first memory with:
   MEAT_MEMORY_CONFIG=./config/app.toml ./bin/memory-cli remember --scope-id scp_release_v1_quickstart --title "Release V1 install smoke" --body "Meat Memory release-V1 installer wrote this quickstart memory." --memory-kind procedure --json
   MEAT_MEMORY_CONFIG=./config/app.toml ./bin/memory-cli search "quickstart memory" --scope-id scp_release_v1_quickstart --limit 5 --json
4. Start service with ./bin/memory-app
5. Export agent skills with:
   ./bin/memory-cli skills export --target all --output-dir ./dist/agent-skills --force

Docs
- README.md
- docs/runbook/usage-guide.md
- docs/architecture-design/install-package-deploy-plan.md
EOF

tar -C "${DIST_DIR}" -czf "${ARCHIVE_PATH}" "${ASSET_NAME}"

if command -v sha256sum >/dev/null 2>&1; then
  sha256sum "${ARCHIVE_PATH}" > "${ARCHIVE_PATH}.sha256"
else
  shasum -a 256 "${ARCHIVE_PATH}" > "${ARCHIVE_PATH}.sha256"
fi

echo "Built ${ARCHIVE_PATH}"
echo "Checksum ${ARCHIVE_PATH}.sha256"
