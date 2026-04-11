---
name: claude-code-meat-memory
description: Use this skill when Claude Code, TRAE, or Qoder should connect to Meat Memory for codebase memory, durable decisions, task context search, and project scope promotion via CLI or MCP.
---

# Claude Code / TRAE / Qoder Meat Memory

## Purpose

This skill helps code-collaboration Agents use Meat Memory as a durable project context layer.

Use it for:

- Loading previous architecture decisions.
- Remembering coding constraints and user preferences.
- Recording integration notes after implementation.
- Promoting useful user-scope memories into project scope.

## Startup Routine

Run these checks in the project root:

```bash
memory-cli config check
memory-cli mcp info
memory-cli key create --name claude-code-local --source skill --scope-kind personal --storage all --json
```

If MCP is enabled, prefer MCP tools for text memory. Use CLI as a fallback.
If key enforcement is enabled, use the returned `raw_key` as `MEAT_MEMORY_KEY` or pass it in `X-Meat-Memory-Key`.

## Scope Guidance

- Use `scp_<project>` for shared project memory.
- Use `scp_user_<name>` only for personal notes.
- Promote from user to project scope when a memory affects the whole repository.

## Recommended Tool Use

- Before a non-trivial code change: call `memory.fetch_context` with the task topic.
- After a durable decision: call `memory.remember` with `memory_kind = "decision"`.
- After discovering a recurring rule: call `memory.remember` with `memory_kind = "constraint"` or `memory_kind = "procedure"`.
- After a risky finding: call `memory.remember` with `memory_kind = "risk"`.

## CLI Fallback

```bash
memory-cli search "配置 TUI MCP CLI" --scope-id scp_meat_memory_v1 --limit 5 --json
```

```bash
memory-cli remember \
  --scope-id scp_meat_memory_v1 \
  --title "Agent 接入偏好" \
  --body "Claude Code / TRAE / Qoder 优先通过 MCP 拉取文本上下文，图片补录可走 CLI 或 HTTP。" \
  --memory-kind preference \
  --visibility project \
  --sensitivity internal \
  --json
```
