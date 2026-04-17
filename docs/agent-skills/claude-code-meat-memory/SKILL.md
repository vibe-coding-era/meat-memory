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

- Before reading or writing memory for a workspace, confirm the project memory boundary with numbered choices.
- Use `scp_<project>` for shared project memory.
- Use `scp_user_<name>` only for personal notes.
- Promote from user to project scope when a memory affects the whole repository.

## V2.6 Project Boundary Check

When a project appears through a new repo, session, MCP client, CLI task, or imported working directory, ask with numbers first:

1. 是否为新项目
   1. 是，新项目
   2. 否，已有项目
2. If it is new, ask whether memory should interoperate with other projects:
   1. 互通
   2. 不互通，完全隔离
3. If it is new, ask whether this is team or personal memory:
   1. 团队记忆
   2. 个人记忆
4. If it is not new, ask how to select the boundary:
   1. 列出现有记忆列表
   2. 手动输入一个 `scope_id`

Prefer:

```bash
memory-cli tui project-init --interactive
memory-cli project init --interactive
```

Use the returned `scope_id` for `search`, `remember`, context, and docs commands. If the helper creates a new project, use the returned `raw_key`; if an existing project is selected, keep using the existing key for that scope. Do not write to the previous default scope unless the user selected it.

## Recommended Tool Use

- Before a non-trivial code change: call `memory.fetch_context` with the task topic.
- After a durable decision: call `memory.remember` with `memory_kind = "decision"`.
- After discovering a recurring rule: call `memory.remember` with `memory_kind = "constraint"` or `memory_kind = "procedure"`.
- After a risky finding: call `memory.remember` with `memory_kind = "risk"`.
- During an active coding session: call `memory.context.upsert` or `memory-cli context upsert` for short-term task state.
- When project docs matter: call `memory.docs.sync` / `memory.docs.search` or `memory-cli docs status/sync/list` for mid-term project documents.
- When using multiple integration surfaces: create a source and attach multiple keys with `memory-cli source key-create`.

## V2.4 Project Context Commands

Short-term context:

```bash
memory-cli context upsert \
  --session-id claude-code-session \
  --task-id current-change \
  --title "Current coding context" \
  --body "Claude Code is collecting local build and review notes." \
  --labels claude-code,short-term \
  --json
```

Project document source and sync:

```bash
memory-cli source create \
  --name "Repository docs" \
  --source-kind local_docs \
  --sync-mode index_only \
  --local-root ./docs \
  --json
```

```bash
memory-cli docs status --source-id src_... --json
memory-cli docs sync --source-id src_... --dry-run --json
memory-cli docs conflicts --source-id src_... --json
```

If conflicts or missing documents are reported, stop and surface them to the user instead of overwriting local files.

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
