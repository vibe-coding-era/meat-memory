#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EXPORT_DIR="/tmp/meat-memory-v21-acceptance-skills"

cd "${ROOT_DIR}"

echo "[v2.1] checking CLI binary tests"
cargo test -p memory-cli --bin memory-cli --quiet

echo "[v2.1] checking configuration summary"
cargo run -p memory-cli -- config check

echo "[v2.1] checking MCP summary"
cargo run -p memory-cli -- mcp info

echo "[v2.1] checking MCP summary with HTTP connectivity"
cargo run -p memory-cli -- mcp info --check-http

echo "[v2.1] checking TUI init preview"
cargo run -p memory-cli -- tui init

echo "[v2.1] checking interactive TUI init"
printf '1\n2\ny\n\n\n\n\n\n\n\n\n\n\ny\n' | cargo run -p memory-cli -- tui init --interactive

echo "[v2.1] checking interactive TUI init in English"
printf '2\n1\nn\n\n\n\n\n\n\n\n\n\n\ny\n' | cargo run -p memory-cli -- tui init --interactive

echo "[v2.1] exporting all agent skills"
cargo run -p memory-cli -- skills export --target all --output-dir "${EXPORT_DIR}" --force

echo "[v2.1] validating exported bundle structure"
test -f "${EXPORT_DIR}/README.md"
test -f "${EXPORT_DIR}/codex-meat-memory/SKILL.md"
test -f "${EXPORT_DIR}/codex-meat-memory/agents/openai.yaml"
test -f "${EXPORT_DIR}/codex-meat-memory/assets/icon.svg"
test -f "${EXPORT_DIR}/claude-code-meat-memory/SKILL.md"
test -f "${EXPORT_DIR}/claude-code-meat-memory/agents/openai.yaml"
test -f "${EXPORT_DIR}/claude-code-meat-memory/assets/icon.svg"
test -f "${EXPORT_DIR}/execution-agent-meat-memory/SKILL.md"
test -f "${EXPORT_DIR}/execution-agent-meat-memory/agents/openai.yaml"
test -f "${EXPORT_DIR}/execution-agent-meat-memory/assets/icon.svg"

echo "[v2.1] acceptance completed"
