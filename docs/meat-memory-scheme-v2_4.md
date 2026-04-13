# Meat Memory V2.4 方案：短期与中期 Memory 能力

更新时间：2026-04-11

状态：Planning Draft

## 1. 背景

`Meat Memory` 当前已经完成 V1、V2、V2.1、V2.2 与 V2.3 的主体闭环，具备长期记忆、图片记忆、PG + Markdown 双存储、多 scope、key 隔离、混合检索与安全基线。

V2.4 的新增需求是：

1. 允许管理 Agent 实时上下文、项目文档；如果文档本地也有，需要保持同步。
2. 每个来源允许多个 key。

这意味着系统需要从“长期 Memory 为主”扩展到“短期实时上下文 + 中期项目文档 + 长期稳定知识”的生命周期分层，同时把 V2.2 中较粗粒度的 `source_kind` 升级为可管理的 `MemorySource`，让同一个来源实例可以挂载多个 access key。

## 2. 目标

V2.4 聚焦四个方向：

1. 建立短期 Memory 能力，用于 Agent 当前会话、当前任务、实时上下文和工具结果摘要。
2. 建立中期 Memory 能力，用于项目文档、任务文档、设计文档、README、runbook 和 API 文档等可同步工作集。
3. 建立本地项目文档同步能力，能发现新增、变更、删除与冲突，默认不静默覆盖本地文件。
4. 将来源建模为一等对象，支持每个来源绑定多个 key，并兼容现有 key/source_kind 语义。

## 3. 非目标

- 不重写现有长期 `Memory` 主链路。
- 不把所有实时上下文直接塞进长期记忆，避免长期检索被噪音污染。
- 不在默认策略下自动覆盖本地项目文档。
- 不把 source 设计成新的权限边界替代品；权限与隔离仍由 access key、scope 和 policy 共同承担。
- 不提前实现 V3 的音频/视频全多模态范围。

## 4. 分层模型

建议新增 `MemoryLayer`，用于表达生命周期层级：

| Layer | 定位 | 默认保留策略 | 典型内容 |
|---|---|---|---|
| `short_term` | Agent 实时上下文 | TTL / session 关闭后可过期 | 当前任务状态、实时约束、terminal/tool 摘要 |
| `mid_term` | 项目文档与工作集 | 按 source/file 持续同步 | README、设计文档、任务文档、runbook、API 文档 |
| `long_term` | 稳定长期知识 | 现有 Memory 生命周期 | 决策、约束、流程、风险、稳定事实 |

`MemoryLayer` 不替代 `MemoryKind`。`MemoryKind` 继续描述内容类型，例如 `Decision / Procedure / Summary`；`MemoryLayer` 描述内容的生命周期与检索权重。

## 5. 核心对象设计

### 5.1 MemorySource

`MemorySource` 表示一个具体来源实例，而不是 `cli / mcp / skill / http / tui / custom` 这种粗粒度来源类型。

建议字段：

```text
source_id
source_kind
display_name
owner_principal_id
owner_scope_id
source_uri
sync_mode
local_root
status
created_at
updated_at
```

字段语义：

- `source_id`：来源实例公开标识。
- `source_kind`：兼容现有 `KeySourceKind`，如 `cli / mcp / skill / http / tui / custom`。
- `display_name`：用户可读名称，如 `codex-desktop-local`、`project-docs-meat-memory`。
- `owner_principal_id`：来源所属用户或服务身份。
- `owner_scope_id`：默认归属 scope。
- `source_uri`：来源 URI，如 `agent://codex/local`、`file:///Users/Rou/dev_projects/meat-memory/docs`。
- `sync_mode`：同步策略，如 `read_only / index_only / two_way`。
- `local_root`：本地文档根路径，可为空。
- `status`：`active / disabled / archived`。

一个 `MemorySource` 可以关联多个 `AccessKey`。建议给 `access_keys` 增加可空 `source_id` 字段，并保留 `source_kind` 以兼容旧路径。

### 5.2 AgentContext

`AgentContext` 表示短期实时上下文，适合当前 session/task 内快速读写，不默认进入长期 memory。

建议字段：

```text
context_id
source_id
key_id
scope_id
session_id
task_id
layer = short_term
title
body
labels
expires_at
created_at
updated_at
```

关键规则：

- 默认绑定当前 `source_id`、`key_id` 和 `scope_id`。
- `expires_at` 用于过期清理。
- 可通过 promote 操作沉淀为长期 `Memory`。
- 默认只参与当前 session/task 的 context fetch，不参与全局长期检索。

### 5.3 ProjectDocument

`ProjectDocument` 表示中期项目文档索引与同步状态。

建议字段：

```text
document_id
source_id
scope_id
local_path
canonical_uri
title
content_hash
last_seen_mtime
sync_state
conflict_state
artifact_id
memory_id
created_at
updated_at
```

关键规则：

- `local_path` 表示本地文件路径。
- `canonical_uri` 用于统一本地、远端或虚拟文档来源。
- `content_hash` 和 `last_seen_mtime` 用于变更检测。
- `artifact_id` 指向原始文档 artifact。
- `memory_id` 可选，指向从文档提炼出的中期/长期 memory。
- `sync_state` 描述 `clean / changed / deleted / missing / conflicted` 等状态。

## 6. 同步策略

默认策略建议偏保守：

- 本地存在文档时，先做 hash/mtime 比对。
- 无冲突时同步到 artifact 与中期文档索引。
- 系统侧生成内容不直接覆盖本地文档，除非用户显式允许 writeback。
- 本地和系统侧同时变化时进入 `conflicted`，输出冲突报告。
- 删除事件默认只标记，不立即删除长期记忆或历史 artifact。

建议同步模式：

| sync_mode | 行为 |
|---|---|
| `read_only` | 只读本地文档，写入索引，不写回本地 |
| `index_only` | 只维护 searchable index 与 artifact，不生成长期 memory |
| `two_way` | 允许显式写回，但必须有冲突检测和确认入口 |

V2.4 MVP 推荐默认使用 `read_only` 或 `index_only`，把 `two_way` 留作受控增强。

## 7. 检索策略

`fetch_context` 和 `search` 建议支持 layer-aware 合并：

```text
Agent 当前请求:
  short_term(session/task) -> mid_term(project docs) -> long_term(scope memory)

普通长期搜索:
  long_term -> mid_term，可选 short_term
```

建议权重：

- `short_term`：当前 session/task 权重最高，但受 TTL 和 scope/key 限制。
- `mid_term`：项目文档权重中等，按 source、path、mtime、hash 和标题匹配调整。
- `long_term`：稳定知识权重最高，但不一定压过当前显式上下文。

## 8. 接口设计

### 8.1 HTTP

建议新增：

```text
POST /api/v1/sources
GET /api/v1/sources
GET /api/v1/sources/{source_id}
GET /api/v1/sources/{source_id}/keys
POST /api/v1/sources/{source_id}/keys

POST /api/v1/contexts
GET /api/v1/contexts
POST /api/v1/contexts/{context_id}/promote
DELETE /api/v1/contexts/{context_id}

POST /api/v1/documents/sync/preview
POST /api/v1/documents/sync
GET /api/v1/documents
GET /api/v1/documents/conflicts
```

### 8.2 CLI

建议新增：

```text
memory-cli source create --name codex-local --kind cli --scope scp_meat_memory_v1
memory-cli source list
memory-cli source keys --source-id src_xxx
memory-cli key create --source-id src_xxx --name codex-local-primary

memory-cli context upsert --session-id ... --task-id ... --title ... --body ...
memory-cli context list --session-id ...
memory-cli context promote --context-id ctx_xxx

memory-cli docs sync --source-id src_xxx --path docs/
memory-cli docs status --source-id src_xxx
memory-cli docs conflicts --source-id src_xxx
```

### 8.3 MCP

建议新增工具：

```text
memory.context.upsert
memory.context.list
memory.context.promote
memory.docs.sync
memory.docs.search
memory.docs.conflicts
memory.source.list
memory.source.keys
```

## 9. 安全与权限

V2.4 继续沿用 V2.2/V2.3 的安全边界：

- source 不是权限替代品，所有读写仍必须经过 key 和 scope 策略。
- source 下多个 key 仍必须各自独立 hash、状态、storage mode 与 isolation group。
- team key 仍不能读取 personal private scope。
- 短期上下文必须绑定 scope/key/session/task，避免跨 session 泄露。
- 项目文档同步必须限制路径根，避免路径穿越与跨项目读取。
- 文档 writeback 必须显式授权，默认不自动覆盖本地文件。

## 10. 交付物

- V2.4 方案文档与 TaskList。
- `MemoryLayer / MemorySource / AgentContext / ProjectDocument` 领域模型。
- V2.4 数据迁移。
- Source 多 key 管理链路。
- 短期 Agent Context 管理链路。
- 中期 Project Document 同步与检索链路。
- HTTP / MCP / CLI 接口扩展。
- Agent skill 文案更新。
- 监控与统计扩展。
- V2.4 验收脚本与测试报告。

## 11. 验收标准

- 同一个 source 能创建和绑定多个 access key。
- 旧的 `source_kind` key 路径保持兼容。
- Agent 可以写入、读取、过期和 promote 短期上下文。
- 项目文档可以从本地路径导入并进入中期检索。
- 本地文档存在时，同步链路能通过 hash/mtime 判断 clean/changed/deleted/conflicted。
- 默认同步策略不会静默覆盖本地文件。
- `fetch_context` 能按短期/中期/长期顺序组合上下文。
- HTTP / MCP / CLI 都具备最小可用入口。
- 安全回归覆盖 source 多 key、短期上下文隔离和文档路径限制。
