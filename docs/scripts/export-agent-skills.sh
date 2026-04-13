#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SOURCE_DIR="$ROOT_DIR/docs/agent-skills"
TARGET="${1:-all}"
OUT_DIR="${2:-$ROOT_DIR/dist/agent-skills}"

usage() {
  cat <<'EOF'
Usage:
  ./docs/scripts/export-agent-skills.sh <target> [output-dir]

Targets:
  codex
  claude-code
  execution-agent
  all

Examples:
  ./docs/scripts/export-agent-skills.sh codex /tmp/skills
  ./docs/scripts/export-agent-skills.sh all ./dist/agent-skills
EOF
}

copy_skill() {
  local source_name="$1"
  local dest_name="$2"
  mkdir -p "$OUT_DIR/$dest_name"
  cp "$SOURCE_DIR/$source_name/SKILL.md" "$OUT_DIR/$dest_name/SKILL.md"
  if [ -d "$SOURCE_DIR/$source_name/agents" ]; then
    mkdir -p "$OUT_DIR/$dest_name/agents"
    cp "$SOURCE_DIR/$source_name/agents/"* "$OUT_DIR/$dest_name/agents/"
  fi
  if [ -d "$SOURCE_DIR/$source_name/assets" ]; then
    mkdir -p "$OUT_DIR/$dest_name/assets"
    cp "$SOURCE_DIR/$source_name/assets/"* "$OUT_DIR/$dest_name/assets/"
  fi
}

write_bundle_readme() {
  cat > "$OUT_DIR/README.md" <<EOF
# Meat Memory Agent Skills Bundle

This bundle was exported from:

- source: \`$SOURCE_DIR\`
- generated_at: $(date '+%Y-%m-%d %H:%M:%S %Z')
- target: \`$TARGET\`

Included skills:

$(find "$OUT_DIR" -mindepth 2 -maxdepth 2 -name SKILL.md | sort | sed "s#$OUT_DIR/##; s#/SKILL.md#:#")

Suggested next steps:

1. Run \`memory-cli config check\`
2. Run \`memory-cli mcp info\`
3. Copy the exported skill folder into the target Agent platform's skill or instruction directory
4. Configure MCP with:
   - tools url: \`http://127.0.0.1:8080/mcp/tools\`
   - call url: \`http://127.0.0.1:8080/mcp/tools/call\`
EOF
}

case "$TARGET" in
  codex)
    rm -rf "$OUT_DIR"
    copy_skill "codex-meat-memory" "codex-meat-memory"
    ;;
  claude-code)
    rm -rf "$OUT_DIR"
    copy_skill "claude-code-meat-memory" "claude-code-meat-memory"
    ;;
  execution-agent)
    rm -rf "$OUT_DIR"
    copy_skill "execution-agent-meat-memory" "execution-agent-meat-memory"
    ;;
  all)
    rm -rf "$OUT_DIR"
    copy_skill "codex-meat-memory" "codex-meat-memory"
    copy_skill "claude-code-meat-memory" "claude-code-meat-memory"
    copy_skill "execution-agent-meat-memory" "execution-agent-meat-memory"
    ;;
  -h|--help|help)
    usage
    exit 0
    ;;
  *)
    echo "Unsupported target: $TARGET" >&2
    usage >&2
    exit 1
    ;;
esac

write_bundle_readme

echo "Exported agent skills to $OUT_DIR"
