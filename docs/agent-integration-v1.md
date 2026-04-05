# V1 Agent 接入指南

本文聚焦 V1 的实际可用接入面，不讲抽象愿景，只讲当前仓库已经提供或明确约定的接法。

## 1. V1 接入原则

- 首选 `MCP over HTTP`：当 Agent 或 Agent 平台支持 MCP 工具协议时，优先接 `GET /mcp/tools` 与 `POST /mcp/tools/call`。
- 次选 `HTTP API`：当上游更适合显式 REST 调用时，接 `POST /api/v1/memories`、`POST /api/v1/images`、`POST /api/v1/context/search`。
- 补充 `CLI`：当接入方更适合本机脚本或本地自动化时，使用 `memory-cli remember`、`memory-cli remember-image`、`memory-cli search`。
- 文件投影保底：Markdown projection 会把记忆投影到 `MEMORY.md`，适合人工巡检、调试和后续 Agent 文件侧车扩展。

## 2. V1 能力边界

- 文本记忆：HTTP / CLI / MCP 全支持。
- 图片记忆：HTTP / CLI 已支持；MCP 在 V1 仍保持文本优先，不直接暴露图片工具。
- 中文优先：抽取、caption、图谱样例以中文为主。
- 模型兼容：通过 provider/model/route registry 抽象兼容 Gemini、Claude、ChatGPT、千问、豆包、MiniMax、GLM。

## 3. Agent 适配矩阵

| Agent / 平台 | V1 推荐接法 | 读写方式 | 备注 |
|---|---|---|---|
| Codex | MCP over HTTP，或本机 CLI | 写入文本/图片、检索上下文 | 适合本地开发机直连 `memory-cli serve` 或 `memory-app` |
| Claude Code | MCP over HTTP | 写入文本、检索上下文、发布记忆 | 若上游更偏命令式，也可调用 CLI |
| TRAE | HTTP API + CLI | 写入文本/图片、检索上下文 | 若后续平台 MCP 能力稳定，可切 MCP |
| Qoder | MCP over HTTP + Markdown 投影 | 文本记忆、上下文召回 | 适合“工具调用 + 文件投影”双轨 |
| OpenClaw | MCP over HTTP | 文本记忆、上下文召回 | 需要时可在其编排层包装 HTTP API |
| CoWork | MCP over HTTP 或 REST Connector | 文本记忆、搜索上下文 | 适合云上统一接入 |
| QoderWork | MCP over HTTP + CLI | 文本记忆、检索、图片补录 | 批处理场景可走 CLI |

上表是 Meat Memory 的“推荐接法”，不是第三方产品官方能力承诺。

## 4. 本地启动

### 4.1 启 HTTP + MCP

```bash
cp .env.example .env
./scripts/verify.sh
./scripts/bootstrap.sh
./scripts/dev-up.sh
```

默认 `compose` 会启动：

- `pgvector`
- `memory-app`
- `memory-worker`

V1 中 `config/docker.toml` 已开启 `enable_mcp = true`，因此应用会同时暴露：

- `GET /mcp/tools`
- `POST /mcp/tools/call`
- `POST /api/v1/memories`
- `POST /api/v1/images`
- `POST /api/v1/context/search`

### 4.2 用本机 CLI 启服务

如果你不想走容器：

```bash
cargo run -p memory-cli -- serve --bind 127.0.0.1:8080
```

是否挂出 MCP 由配置文件中的 `features.enable_mcp` 决定。`config/local.example.toml` 默认已开启。

## 5. MCP 接入

### 5.1 列工具

```bash
curl http://127.0.0.1:8080/mcp/tools
```

### 5.2 调 `memory.remember`

```bash
curl -X POST http://127.0.0.1:8080/mcp/tools/call \
  -H 'content-type: application/json' \
  -d '{
    "name": "memory.remember",
    "arguments": {
      "scope_id": "scp_agent_demo",
      "title": "代码评审偏好",
      "body": "用户偏好先看风险，再看总结。",
      "memory_kind": "preference"
    }
  }'
```

### 5.3 调 `memory.search`

```bash
curl -X POST http://127.0.0.1:8080/mcp/tools/call \
  -H 'content-type: application/json' \
  -d '{
    "name": "memory.search",
    "arguments": {
      "scope_id": "scp_agent_demo",
      "query": "评审偏好 风险",
      "limit": 5
    }
  }'
```

## 6. HTTP 接入

适合不想做 MCP 适配、只想直接调接口的 Agent 平台。

- 文本写入：`POST /api/v1/memories`
- 图片写入：`POST /api/v1/images`
- 上下文搜索：`POST /api/v1/context/search`

详见 [docs/api/http-api-v1.md](./api/http-api-v1.md)。

## 7. CLI 接入

适合：

- 本地开发机自动化
- Shell workflow
- CI/CD 阶段的记忆补录
- 无需长期服务驻留的批处理任务

示例：

```bash
cargo run -p memory-cli -- remember \
  --scope-id scp_agent_demo \
  --title "提交规范" \
  --body "提交消息使用 Conventional Commits。" \
  --memory-kind procedure \
  --json
```

```bash
cargo run -p memory-cli -- search "提交规范" \
  --scope-id scp_agent_demo \
  --json
```

图片写入：

```bash
cargo run -p memory-cli -- remember-image \
  --scope-id scp_agent_demo \
  --title "登录页截图" \
  --body "Codex 登录页截图" \
  --file ./samples/login.png \
  --json
```

详见 [docs/api/cli-v1.md](./api/cli-v1.md)。

## 8. 对接建议

- 对 Codex / Claude Code 这类本地代码 Agent：优先本地 `memory-cli serve` 或 `memory-app`，保持同机低延迟。
- 对 OpenClaw / CoWork / QoderWork 这类编排层：优先统一接 HTTP MCP，便于集中鉴权和路由。
- 对图片场景：V1 先走 HTTP/CLI，等 V2/V3 再扩展 MCP 图片工具。
- 对 Markdown projection：把它当“可审计侧车”，不要把它当唯一主存；主存仍应是 PG。
