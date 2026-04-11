# V1 MCP 工具面

## 1. 当前实现状态

V1 已提供 MCP HTTP transport：

- `GET /mcp/tools`
- `POST /mcp/tools/call`

当配置文件开启 `features.enable_mcp = true` 时：

- `memory-app`
- `memory-cli serve`

都会把以上路由挂出来。

## 2. 工具清单

| Tool | 用途 |
|---|---|
| `memory.remember` | 写入文本记忆 |
| `memory.fetch_context` | 搜索并返回图谱增强上下文 |
| `memory.search` | 搜索上下文 |
| `memory.publish` | 提升既有记忆的可见级别 |
| `memory.promote` | 跨 scope 发布并按策略进入 active / candidate |

V1 说明：

- MCP 当前主打文本闭环。
- 图片记忆在 V1 走 HTTP / CLI。

## 3. 工具发现

```bash
curl http://127.0.0.1:8080/mcp/tools
```

也可以通过 CLI 查看当前配置下的 MCP 信息：

```bash
cargo run -p memory-cli -- mcp info
cargo run -p memory-cli -- mcp info --json
cargo run -p memory-cli -- mcp info --check-http
```

返回里会包含：

- `service`
- `version`
- `transports`
- `tools`

如果你已经启动了本地服务，还可以用 `--check-http` 额外确认：

- 当前 `http://127.0.0.1:8080/mcp/tools` 是否可达
- 返回是否是预期的 `HTTP 200`

## 4. `memory.remember`

请求：

```json
{
  "name": "memory.remember",
  "arguments": {
    "scope_id": "scp_demo_mcp",
    "title": "代码评审偏好",
    "body": "评审先列风险，再列摘要。",
    "artifact_kind": "message",
    "memory_kind": "preference",
    "visibility": "private",
    "sensitivity": "internal"
  }
}
```

## 5. `memory.search`

请求：

```json
{
  "name": "memory.search",
  "arguments": {
    "scope_id": "scp_demo_mcp",
    "query": "评审 风险",
    "limit": 5
  }
}
```

返回中包含：

- `memory_count`
- `entity_count`
- `relation_count`
- `memories`
- `entities`
- `relations`

## 6. `memory.fetch_context`

`memory.fetch_context` 与 `memory.search` 的输入类似，但在工具语义上更强调给 Agent 拉上下文包。

## 7. `memory.publish`

请求：

```json
{
  "name": "memory.publish",
  "arguments": {
    "scope_id": "scp_demo_mcp",
    "memory_id": "mem_01...",
    "target_visibility": "team"
  }
}
```

## 8. `memory.promote`

请求：

```json
{
  "name": "memory.promote",
  "arguments": {
    "source_scope_id": "scp_user_alice",
    "memory_id": "mem_01...",
    "source_scope_type": "user",
    "target_scope_id": "scp_project_demo",
    "target_scope_type": "project",
    "target_visibility": "project"
  }
}
```

返回中包含：

- `scope_id`
- `owner_scope_id`
- `published_from_scope_id`
- `memory_state`
- `visibility`

## 9. 错误模型

统一返回：

```json
{
  "error": {
    "code": "invalid_arguments",
    "message": "..."
  }
}
```

当前主要错误码：

- `invalid_arguments`
- `unsupported_tool`
- `not_found`
- `forbidden`
- `internal_error`

## 10. 接入建议

- 需要标准化工具发现与调用的 Agent 平台，优先走 MCP。
- 需要图片写入的场景，MCP + HTTP 双轨最实用：文本/检索走 MCP，图片补录走 HTTP。
