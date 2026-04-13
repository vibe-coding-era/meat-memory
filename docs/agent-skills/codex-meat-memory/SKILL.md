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

1. Search before writing if the user asks for historical context.
2. Use a stable scope id for the project, for example `scp_meat_memory_v1`.
3. Write concise memories as facts, preferences, decisions, procedures, constraints, risks, summaries, or insights.
4. Add source refs when possible, such as `workspace://path/to/file` or `thread://current`.
5. Promote only when memory should move from a personal/user scope into a project/team scope.

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
