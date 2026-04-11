#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "${ROOT_DIR}"

echo "[acceptance] running kernel flow"
cargo test -p memory-kernel --test kernel_flow_tests -- --test-threads=1

echo "[acceptance] running HTTP flow"
cargo test -p memory-http --test http_api_tests -- --test-threads=1

echo "[acceptance] running MCP flow"
cargo test -p memory-mcp --test mcp_tools_tests -- --test-threads=1

echo "[acceptance] running CLI flow"
cargo test -p memory-cli --test cli_e2e -- --test-threads=1
