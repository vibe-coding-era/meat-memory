# V2 Bilingual And Sharing Acceptance

本文件用于覆盖 V2 的三类验收入口：

- 中英双语写入、抽取、检索
- user/project/team scope promotion 与 review/redaction
- 持久化 sync/merge/conflict 保留

## 1. Bilingual Memory Extraction

### 1.1 English decision

输入：

```text
We decided to keep PostgreSQL as the canonical store for durable memory.
```

期望：

- `memory_kind = decision`
- `language_code = en`
- 可以抽出 `PostgreSQL`

### 1.2 Chinese procedure

输入：

```text
发布流程：先执行 smoke，再执行回滚检查。
```

期望：

- `memory_kind = procedure`
- `language_code = zh-CN`

### 1.3 Mixed entity graph

输入：

```text
project Meat Memory uses PostgreSQL。团队平台组维护 Gateway API。
```

期望：

- 能抽出 `Meat Memory`、`PostgreSQL`、`平台组`、`Gateway API`
- 关系抽取继续覆盖英文 `uses` 与中文上下文实体

## 2. Scope Promotion

### 2.1 User to project

输入 memory：

```text
scope = user
visibility = private
sensitivity = restricted
body = token: abc123 alice@example.com
```

动作：

- 调用 HTTP `/api/v1/memories/promote`
- 或 MCP `memory.promote`

期望：

- 目标 scope 生成新 memory
- `owner_scope_id` 保留原 user scope
- `published_from_scope_id` 指向原 scope
- 因敏感级较高进入 `candidate`
- 内容发生 redaction

### 2.2 Project to team

输入 memory：

```text
scope = project
visibility = project
sensitivity = internal
```

期望：

- 允许 promotion 到 `team`
- 新 memory 可以直接进入 `active`

## 3. Persistent Sync

### 3.1 File replication engine

动作：

- 追加 oplog entry
- 重启 engine

期望：

- entry 仍存在
- `SyncStatus.entry_count` 保持不丢

### 3.2 Conflict retention

动作：

- 对同一 object 写入 divergent version

期望：

- `merge_ops` 返回 `Conflict`
- `ConflictRecord` 被保留
- `SyncStatus.conflict_count` 增加
