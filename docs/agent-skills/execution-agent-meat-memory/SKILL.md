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

1. Confirm the project memory boundary before fetching or writing workflow memory.
2. Fetch context at the start of a workflow.
3. Record durable outcomes at the end of each major step.
4. Store handoff notes as `summary` or `procedure`.
5. Store irreversible operational risks as `risk`.
6. Promote memory only after confirming it belongs to the shared project/team scope.
7. Store active workflow state in short-term context with `memory.context.upsert` or `memory-cli context upsert`.
8. Sync project runbooks and task docs as mid-term memory with `memory.docs.sync` or `memory-cli docs sync`.

## V2.6 Project Boundary Check

For every new workflow project, imported runbook set, execution workspace, or MCP/CLI entry path, ask with numbered choices before using memory:

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

Prefer the helper:

```bash
memory-cli tui project-init --interactive
memory-cli project init --interactive
```

Use the returned `scope_id` and key for all workflow context, docs, and durable memory commands. Do not share an execution workflow into an old project scope unless the user selected that scope.

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
