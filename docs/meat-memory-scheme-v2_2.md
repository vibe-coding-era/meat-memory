# Meat Memory Scheme V2.2

状态：Draft

更新时间：2026-04-11

主题：增强用户端使用体验，围绕 key、权限隔离、存储模式、跨 key 知识图谱和监控面板展开。

## 1. 背景

V2.2 面向真实用户侧接入体验。用户可以通过 MCP、CLI、Skills 使用 Meat Memory，但在正式使用前必须先申请 key。每个用户可以拥有多个 key，用于区分不同来源、不同 Agent、不同存储模式和不同隔离策略。

当前系统已经具备以下基础：

- `Scope` 支持 user、team、project、organization 等记忆归属边界。
- `Policy` 支持 visibility、sensitivity、publish、promote 和基础脱敏。
- Kernel 支持 PostgreSQL 与 Markdown 双后端。
- HTTP、CLI、MCP 和 Skill 导出已经形成基本接入面。
- `/metrics` 已有读写和搜索指标快照。

V2.2 不应把 key 设计成 scope 的替代品。推荐新增 `AccessKey / MemoryProfile` 层：key 负责身份、来源、隔离、存储模式和统计；scope 继续负责记忆归属、共享层级和权限边界。

## 2. 设计目标

- 所有 MCP、CLI、Skills、HTTP 写入和查询都能绑定 key。
- 首次安装初始化时自动生成默认 key。
- TUI 支持创建 key，并覆盖名称、来源、团队/个人、存储模式、完全隔离等配置。
- 个人 memory 可以读取团队 memory；团队 key 不能读取个人 memory。
- 同一用户或团队下的多个 key 默认可互通知识，但完全隔离 key 不参与互通。
- 支持 file、vector、all 三种存储模式。
- 建立跨 key 的知识图谱与索引过滤规则。
- 增强监控体系，提供读写性能与 key/source/storage 维度的监控面板。

## 3. 非目标

- 不在 V2.2 MVP 中重写已有 scope/promotion 体系。
- 不把 key 明文保存到数据库。
- 不要求 V2.2 MVP 一次性完成完整向量召回质量优化；vector-only 可以先落 schema、路由和最小检索，再进入 V2.2 Full 优化。
- 不把监控面板做成独立大型前端应用，优先复用现有 HTTP 控制台。

## 4. 核心对象

### 4.1 AccessKey

建议字段：

```text
key_id
key_hash
display_name
source_kind
owner_principal_id
owner_scope_id
scope_kind
storage_mode
is_fully_isolated
isolation_group_id
status
created_at
last_used_at
```

字段语义：

- `key_id`：公开标识，可返回给用户和日志。
- `key_hash`：key 的不可逆哈希，不保存明文 key。
- `display_name`：用户可读名称，例如 `codex-local`、`team-mcp`。
- `source_kind`：来源类型，建议枚举 `cli / mcp / skill / http / tui / custom`。
- `owner_principal_id`：归属用户或服务身份。
- `owner_scope_id`：默认读写 scope。
- `scope_kind`：`personal / team`，用于权限策略。
- `storage_mode`：`file / vector / all`。
- `is_fully_isolated`：完全隔离开关。
- `isolation_group_id`：允许跨 key 互通的逻辑组。

### 4.2 RequestContext

所有入口进入 Kernel 前应构建统一请求上下文：

```text
request_id
key_id
source_kind
principal_id
owner_scope_id
scope_kind
storage_mode
is_fully_isolated
isolation_group_id
```

`RememberTextRequest`、`RememberImageRequest`、`SearchContextRequest` 和 promote/publish 请求应逐步接入 `RequestContext`，用于鉴权、路由、审计、监控和索引过滤。

## 5. 权限规则

### 5.1 个人 key

- 可读自己的 personal scope。
- 可读被授权的 team/project/organization scope。
- 默认写入自己的 personal scope。
- 写入团队 scope 必须显式指定目标 scope，或通过 promote/review 流程发布。

### 5.2 团队 key

- 可读 team/project/organization scope。
- 不可读取任何 user/private personal scope。
- 默认写入团队 scope。
- 不能通过 search、fetch_context、graph expansion 绕过个人隔离。

### 5.3 完全隔离 key

- 只能读写自身 key namespace 或自身 `isolation_group_id`。
- 不参与默认跨 key 图谱扩展。
- 不参与默认跨 key 索引合并。
- 统计仍可归入全局监控，但不能暴露其他 key 的明细数据。

### 5.4 跨 key 互通

推荐默认规则：

```text
同一 user 的多个 personal key：默认互通
同一 team 的多个 team key：默认互通
personal key 读取 team：允许，按 membership + visibility
team key 读取 personal：禁止
不同来源 key 互通：必须 isolation_group_id 相同且 is_fully_isolated=false
```

## 6. 存储模式

### 6.1 file

- 写入 Markdown / 文件投影。
- 不要求写入 PG 向量索引。
- 搜索优先走文件解析和关键词匹配。
- 适合本地、可审阅、人类可改的工作流。

### 6.2 vector

- 写入 PG / vector index。
- 不写 Markdown projection。
- 搜索优先走 embedding / vector planner。
- 适合服务端、Agent 高频检索和大规模召回。

### 6.3 all

- PG + Markdown 双写。
- 搜索走混合 query planner：关键词、向量、图谱扩展、rerank。
- 适合默认安装和多数 Agent 接入。

当前仓库已具备 PG/Markdown 开关、`memory_embeddings`、embedding 写入和 keyword + vector 合并召回。V2.2 Full 后续重点是 graph expansion、rerank 质量优化和 vector-only 端到端验收。

## 7. 数据模型建议

新增迁移建议：`migrations/0005_access_keys.sql`

建议表：

```text
access_keys
key_usage_events
memory_key_links
memory_embeddings
```

`access_keys` 保存 key 元数据和哈希。

`key_usage_events` 保存读写调用事件，用于统计来源、性能、错误率和活跃度。

`memory_key_links` 保存 memory 与 key / isolation group 的关联，用于跨 key 检索过滤。

`memory_embeddings` 保存 memory 向量和 embedding 元数据，用于 vector-only 与 all 模式。

## 8. 接口设计

### 8.1 HTTP

建议新增：

```text
POST /api/v1/keys
GET /api/v1/keys
PATCH /api/v1/keys/{key_id}
POST /api/v1/keys/{key_id}/rotate
GET /api/v1/keys/{key_id}/stats
GET /api/v1/metrics/keys
```

认证头：

```text
Authorization: Bearer mmk_...
X-Meat-Memory-Key: mmk_...
```

兼容策略：

- 无 key 的旧调用在开发模式下可走默认 key。
- 生产模式应要求 key。
- 无 key 兼容调用应返回 warning，提示迁移到 key 认证。

### 8.2 CLI

建议新增：

```text
memory-cli key create --name codex-local --source cli --scope-kind personal --storage all
memory-cli key list
memory-cli key use --raw-key mmk_...
memory-cli key stats
memory-cli tui key-create --name codex-local --source tui --scope-kind personal --storage all
```

现有命令增加：

```text
memory-cli remember --key <key_id>
memory-cli search --key <key_id>
memory-cli serve --require-key
```

环境变量：

```text
MEAT_MEMORY_KEY
MEAT_MEMORY_DEFAULT_KEY_ID
```

### 8.3 MCP

MCP HTTP transport 应从 HTTP header 获取 key。stdio transport 可从环境变量或启动配置获取 key。

MCP 错误建议区分：

```text
unauthorized
forbidden
isolation_denied
invalid_storage_mode
```

### 8.4 Skills

导出的 Skill bundle 增加 key 配置说明：

```text
MEAT_MEMORY_KEY=...
MEAT_MEMORY_BASE_URL=http://127.0.0.1:8080
```

Skill 文案应明确：首次使用前先通过 CLI/TUI 申请 key。

## 9. 监控设计

现有 `/metrics` 保留。V2.2 扩展三个层次：

```text
/metrics
  总体 JSON snapshot

/api/v1/metrics/keys
  按 key/source/storage_mode 聚合的 JSON

/admin/metrics 或现有首页增强
  人可读监控面板
```

指标维度：

```text
operation: remember/search/promote/publish/mcp_call
key_id
source_kind
scope_kind
storage_mode
store: pg/markdown/vector
result: success/failure/empty/hit
latency_ms
```

面板优先展示：

- 总读写量。
- search 命中率。
- write 成功率。
- p50 / p95 / max latency。
- PG / Markdown / Vector 写入占比。
- key 活跃度。
- 慢查询与失败原因。

## 10. TaskList

### V2.2 MVP

| ID | 任务 | 状态 | 说明 |
|---|---|---|---|
| `V2.2-DOC-001` | V2.2 方案与 TaskList 文档 | done | 本文档与版本化任务表收口 |
| `V2.2-DOM-001` | AccessKey 领域模型 | done | 新增 key/source/scope/storage/isolation 枚举与校验 |
| `V2.2-DB-001` | AccessKey 数据迁移 | done | 新增 access_keys、key_usage_events、memory_key_links、memory_embeddings |
| `V2.2-CFG-001` | Config 默认 key 与认证开关 | done | 支持 access 配置、require_key 与环境变量覆盖 |
| `V2.2-POL-001` | key 权限隔离策略 | done | 团队 key 不能访问个人 scope，完全隔离按 isolation_group 过滤 |
| `V2.2-KER-001` | RequestContext 接入 Kernel | done | remember/search/image 带 key context |
| `V2.2-STO-001` | storage_mode 路由 | done | file/vector/all 决定 PG、Markdown、索引写入路径 |
| `V2.2-HTTP-001` | key 管理 HTTP API | done | 已完成 create/list/update/rotate/stats 与 metrics 入口 |
| `V2.2-HTTP-002` | HTTP key 认证 middleware | done | 支持 Bearer 与 X-Meat-Memory-Key |
| `V2.2-CLI-001` | CLI key 子命令 | done | create/list/use/stats |
| `V2.2-TUI-001` | TUI 默认 key 与创建 key 流程 | done | 已支持 `tui key-create`，首次 init 写配置时会创建默认 key 并落到 key 文件 |
| `V2.2-MCP-001` | MCP key 上下文 | done | 支持 tool 参数与 HTTP header |
| `V2.2-SKL-001` | Skill key 接入说明 | done | 导出模板增加 key 环境变量与申请流程 |
| `V2.2-OBS-001` | 按 key/source/storage 统计 | done | 扩展 metrics snapshot 与 keyed operation 统计 |
| `V2.2-QA-001` | V2.2 MVP 验收 | done | domain/observability/kernel/http/cli/mcp/pg store 回归 |

### V2.2 Full

| ID | 任务 | 状态 | 说明 |
|---|---|---|---|
| `V2.2-IDX-001` | memory_embeddings schema | done | 支持 vector-only 与 all 模式的向量数据 |
| `V2.2-IDX-002` | embedding 写入链路 | done | 写入 memory 后生成 1536 维 embedding 并保存到 PG |
| `V2.2-IDX-003` | 混合检索 planner | done | 已完成 keyword + vector + graph expansion 合并召回与基础 rerank |
| `V2.2-KG-001` | 跨 key 知识图谱过滤 | done | 已按 isolation_group 过滤检索，并覆盖 graph expansion 过程中的跨 key 过滤 |
| `V2.2-OBS-002` | 监控面板增强 | done | 已在 HTTP 首页 console 展示搜索/写入延迟、命中率、key 成功率与 storage mode 分布 |
| `V2.2-QA-002` | vector/file/all 模式验收 | done | 已覆盖 file markdown-only、all 模式向量写读与 isolation 过滤，以及 vector-only HTTP/CLI e2e |

## 11. 建议执行顺序

1. `V2.2-DOC-001`
2. `V2.2-DOM-001`
3. `V2.2-DB-001`
4. `V2.2-CFG-001`
5. `V2.2-POL-001`
6. `V2.2-KER-001`
7. `V2.2-STO-001`
8. `V2.2-HTTP-001`
9. `V2.2-HTTP-002`
10. `V2.2-CLI-001`
11. `V2.2-TUI-001`
12. `V2.2-MCP-001`
13. `V2.2-SKL-001`
14. `V2.2-OBS-001`
15. `V2.2-QA-001`
16. `V2.2-IDX-001`
17. `V2.2-IDX-002`
18. `V2.2-IDX-003`
19. `V2.2-KG-001`
20. `V2.2-OBS-002`
21. `V2.2-QA-002`

## 12. 开工建议

建议先按 V2.2 MVP 开工。原因是 key、权限、TUI 和 MCP/CLI 认证已经能显著改善用户端使用体验；完整 vector planner 和跨 key 图谱质量优化可以在 MVP 稳定后继续迭代。
