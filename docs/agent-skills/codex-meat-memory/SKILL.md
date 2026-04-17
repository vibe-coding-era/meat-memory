---
name: codex-meat-memory
description: Use this skill when Codex should read from or write to Meat Memory for long-term project memory, including remember, search, fetch_context, publish, and promote workflows through CLI or MCP.
---

# Codex Meat Memory

## When to Use

Use this skill when working in a project that should persist long-term memory in Meat Memory.

Common triggers:

- The user asks to remember project knowledge.
- The user asks to search previous project context.
- The task needs durable decisions, constraints, preferences, risks, or procedures.
- The user mentions Meat Memory, memory scope, `memory-cli`, or MCP memory tools.

## Quick Checks

Before using memory, inspect the local setup:

```bash
memory-cli config check
memory-cli mcp info
memory-cli key create --name codex-local --source skill --scope-kind personal --storage all --json
```

If `memory-cli` is not on PATH, use:

```bash
cargo run -p memory-cli -- config check
cargo run -p memory-cli -- mcp info
cargo run -p memory-cli -- key create --name codex-local --source skill --scope-kind personal --storage all --json
```

Keep the returned `raw_key` in `MEAT_MEMORY_KEY` or pass it through the MCP HTTP header `X-Meat-Memory-Key`.

## Preferred Workflow

1. Confirm the project memory boundary before reading or writing memory.
2. Search before writing if the user asks for historical context.
3. Use the confirmed scope id for the project instead of defaulting to `scp_meat_memory_v1`.
4. Write concise memories as facts, preferences, decisions, procedures, constraints, risks, summaries, or insights.
5. Add source refs when possible, such as `workspace://path/to/file` or `thread://current`.
6. Promote only when memory should move from a personal/user scope into a project/team scope.

## V2.6 Project Boundary Check

When a project appears through a new workspace, thread, MCP client, CLI session, or any other entry path, ask the user with numbered choices before using long-term memory:

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

Use the TUI or CLI helper when available:

```bash
memory-cli tui project-init --interactive
memory-cli project init --interactive
```

For a new project, use the returned `scope_id` and `raw_key`. For an existing project, use the selected or manually provided `scope_id` and the existing key for that scope. Do not mix project memories by writing to an old default scope.

## V2.4 Short and Mid-Term Memory

Use short-term Agent Context for current-session scratchpads, task state, and tool-result summaries that may later be promoted:

```bash
memory-cli context upsert \
  --session-id codex-session \
  --task-id current-task \
  --title "Current implementation context" \
  --body "Codex is wiring V2.4 Agent Context and Project Documents." \
  --labels codex,v2.4 \
  --json
```

Use project document sources for mid-term memory such as README, design docs, task docs, runbooks, and API docs:

```bash
memory-cli source create \
  --name "Project docs" \
  --source-kind local_docs \
  --sync-mode index_only \
  --local-root ./docs \
  --json
```

```bash
memory-cli docs status --source-id src_... --json
memory-cli docs sync --source-id src_... --dry-run --json
memory-cli docs sync --source-id src_... --json
```

Each source can have multiple keys:

```bash
memory-cli source key-create --source-id src_... --name codex-docs-key --json
```

Do not silently overwrite local project files when conflicts appear. Treat `missing` and `conflicts` as review items and ask for confirmation before applying external changes.

## CLI Examples

Search:

```bash
memory-cli search "release V2 scope promote" --scope-id scp_meat_memory_v1 --limit 5 --json
```

Remember:

```bash
memory-cli remember \
  --scope-id scp_meat_memory_v1 \
  --title "V2.1 执行顺序" \
  --body "V2.1 先做 CLI/MCP 可用性，再做 Agent skill 和 TUI 配置向导。" \
  --memory-kind decision \
  --visibility project \
  --sensitivity internal \
  --json
```

If the local service requires keys, export the key first:

```bash
export MEAT_MEMORY_KEY=mmk_...
```

## MCP Tools

Use these tools when the Agent platform supports MCP:

- `memory.remember`
- `memory.search`
- `memory.fetch_context`
- `memory.publish`
- `memory.promote`
- `memory.context.upsert`
- `memory.context.list`
- `memory.context.promote`
- `memory.context.delete`
- `memory.docs.sync`
- `memory.docs.search`
- `memory.docs.conflicts`

Call `memory.fetch_context` for task setup and `memory.remember` for durable outcomes.
