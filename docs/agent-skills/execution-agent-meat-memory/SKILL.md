---
name: execution-agent-meat-memory
description: Use this skill when OpenClaw, CoWork, QoderWork, or another execution-oriented Agent should use Meat Memory to coordinate tasks, store durable execution notes, and retrieve project context through CLI or MCP.
---

# Execution Agent Meat Memory

## Purpose

This skill is for execution-oriented Agents that run workflows, coordinate tasks, or operate across multiple steps.

Use Meat Memory to persist:

- Execution plans that should survive a session.
- Completed task summaries.
- Operational constraints.
- Cross-agent handoff notes.
- Promotion decisions from personal scope to project/team scope.

## Preflight

```bash
memory-cli config check
memory-cli mcp info
memory-cli key create --name execution-agent-local --source skill --scope-kind team --storage all --json
```

If `config check` reports warnings, fix configuration before writing operational memory.
If key enforcement is enabled, store the returned `raw_key` in the execution environment as `MEAT_MEMORY_KEY`.

## Execution Workflow

1. Fetch context at the start of a workflow.
2. Record durable outcomes at the end of each major step.
3. Store handoff notes as `summary` or `procedure`.
4. Store irreversible operational risks as `risk`.
5. Promote memory only after confirming it belongs to the shared project/team scope.
6. Store active workflow state in short-term context with `memory.context.upsert` or `memory-cli context upsert`.
7. Sync project runbooks and task docs as mid-term memory with `memory.docs.sync` or `memory-cli docs sync`.

## V2.4 Operational Context

Use a source per operational input set and attach multiple keys when different workers need separate credentials:

```bash
memory-cli source create \
  --name "Ops runbooks" \
  --source-kind local_docs \
  --sync-mode index_only \
  --local-root ./docs/runbook \
  --json
```

```bash
memory-cli source key-create --source-id src_... --name execution-worker-key --json
```

Use short-term context for active workflow state:

```bash
memory-cli context upsert \
  --session-id execution-run \
  --task-id deploy-check \
  --title "Deployment check state" \
  --body "Worker has completed config validation and is checking runbooks." \
  --labels execution,runbook \
  --json
```

Use document sync conservatively:

```bash
memory-cli docs status --source-id src_... --json
memory-cli docs sync --source-id src_... --dry-run --json
```

If `conflicts` or `missing` appear, report them as workflow blockers unless the user explicitly approves the next action.

## Example Memory

```bash
memory-cli remember \
  --scope-id scp_meat_memory_v1 \
  --title "V2.1 执行协作规则" \
  --body "执行型 Agent 在开始任务前先 fetch_context，完成后写 summary，遇到共享决策再 promote 到 project scope。" \
  --memory-kind procedure \
  --visibility project \
  --sensitivity internal \
  --json
```

## MCP Tool Preference

- Use `memory.fetch_context` for workflow start.
- Use `memory.remember` for durable execution notes.
- Use `memory.promote` for cross-scope sharing.
- Use `memory.context.upsert/list/promote/delete` for active workflow context.
- Use `memory.docs.sync/search/conflicts` for project runbooks and docs.
