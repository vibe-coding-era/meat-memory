# Meat Memory

`Meat Memory` 是一个纯自研、可自托管的长期 Memory 内核，面向 Agent、模型和本地 / 云部署场景。

当前仓库已经完成 `V1` 交付范围，适合直接本地试跑：

| 能力 | 当前状态 |
|---|---|
| PostgreSQL + Markdown 双存储 | 已支持 |
| 本地独立部署 | 已支持 |
| 云端独立部署骨架 | 已支持 |
| 混合部署预留接口 | 已支持 `sync/oplog/merge` baseline |
| Agent 接入 | 已支持 HTTP / CLI / MCP |
| 多模型抽象 | 已支持 Gemini / Claude / ChatGPT / 千问 / 豆包 / MiniMax / GLM 的 provider catalog 与 route registry |
| LLM 自动切换 | 已支持按 capability 配置 `primary + fallbacks`，主 LLM 不可用时自动切到下一个，并返回提示 |
| 多模态 | `V1` 已支持文本 + 图片 |
| 知识图谱 | 已支持 entity / relation / graph context baseline |
| 中文优先 | 已支持，已补齐系统化中文验收集与回归入口 |

## V2.1 快速入口

安装完成后，如果你想先确认配置、导出 skill，或者快速生成一份可用配置，推荐先走下面这一组命令：

```bash
cargo run -p memory-cli -- config check
cargo run -p memory-cli -- mcp info
cargo run -p memory-cli -- tui init
cargo run -p memory-cli -- skills export --target all --output-dir ./dist/agent-skills --force
```

这组命令分别用于：

- 检查当前配置和模型路由是否可用
- 查看 MCP 是否启用，以及 `/mcp/tools` / `/mcp/tools/call` 的接入地址
- 预览安装后初始化面板，并按需生成推荐配置
- 导出可直接复制给不同 Agent 平台使用的 skill bundle

如果需要数据库连通性检查，可额外执行：

```bash
cargo run -p memory-cli -- config check --database
```

更多命令说明见 [`docs/api/cli-v1.md`](docs/api/cli-v1.md)；skill 包结构与安装说明见 [`docs/agent-skills/README.md`](docs/agent-skills/README.md)。

## 快速开始

如果你只是想先把服务跑起来，推荐直接走 Docker Compose。

### 方案 A：最快试跑

1. 复制环境变量文件。

```bash
cp .env.example .env
```

2. 启动本地完整栈。

```bash
./scripts/dev-up.sh
```

这会启动：

- `pgvector`
- `memory-app`
- `memory-worker`

3. 检查服务是否正常。

```bash
curl http://127.0.0.1:8080/healthz
curl http://127.0.0.1:8080/readyz
curl http://127.0.0.1:8080/api/v1/meta
curl http://127.0.0.1:8080/mcp/tools
```

4. 写入一条文本记忆。

```bash
curl -X POST http://127.0.0.1:8080/api/v1/memories \
  -H 'content-type: application/json' \
  -d '{
    "scope_id": "scp_demo_readme",
    "title": "发布规则",
    "body": "生产变更必须先通过回归测试。",
    "memory_kind": "procedure",
    "visibility": "private",
    "sensitivity": "internal"
  }'
```

5. 搜索刚写入的内容。

```bash
curl -X POST http://127.0.0.1:8080/api/v1/context/search \
  -H 'content-type: application/json' \
  -d '{
    "scope_id": "scp_demo_readme",
    "query": "发布规则 回归测试",
    "limit": 5
  }'
```

6. 停止服务。

```bash
./scripts/dev-down.sh
```

说明：

- `dev-down.sh` 只会 `stop` 容器，不会清空 volume。
- 本地数据会保存在 compose volume 中，不会因为容器重建直接丢失。

### 方案 B：开发模式运行

如果你想本地用 `cargo run` 调试，推荐先只起数据库，再直接跑 Rust 服务。

1. 复制环境变量文件。

```bash
cp .env.example .env
```

2. 先启动本地 PG 开发库。

```bash
./scripts/dev-db-up.sh
```

3. 安装 Rust 组件并初始化本地开发环境。

```bash
./scripts/bootstrap.sh
```

4. 运行环境校验。

```bash
./scripts/verify.sh
```

注意：

- `verify.sh` 会检查 `pgvector` 是否可连通。
- 所以第一次试跑时，应该先执行 `./scripts/dev-db-up.sh`，再执行 `./scripts/verify.sh`。

5. 跑测试。

```bash
cargo test --workspace --lib --bins --quiet
```

6. 启动服务。

```bash
cargo run -p memory-app
```

或者：

```bash
cargo run -p memory-cli -- serve --bind 127.0.0.1:8080
```

7. 调试完成后停止数据库。

```bash
./scripts/dev-db-down.sh
```

## 一次完整试用

### 用 CLI 写入文本记忆

```bash
cargo run -p memory-cli -- remember \
  --scope-id scp_cli_demo \
  --title "评审规则" \
  --body "代码评审先列风险，再列摘要。" \
  --memory-kind preference \
  --json
```

### 用 CLI 搜索

```bash
cargo run -p memory-cli -- search "评审 风险" \
  --scope-id scp_cli_demo \
  --limit 5 \
  --json
```

### 用 CLI 写入图片记忆

```bash
cargo run -p memory-cli -- remember-image \
  --scope-id scp_cli_demo \
  --title "登录页截图" \
  --body "Codex 登录页截图" \
  --file /path/to/your-image.png \
  --json
```

图片返回里会包含：

- `asset_uri`
- `vision_caption`
- `vision_model_alias`

说明：

- 请把 `/path/to/your-image.png` 换成你机器上的真实图片路径。

## HTTP 与 MCP 入口

默认服务地址：

```text
http://127.0.0.1:8080
```

浏览器打开 `http://127.0.0.1:8080/` 可直接访问内置 Browser Console。

### HTTP 路由

| 路由 | 方法 | 用途 |
|---|---|---|
| `/` | `GET` | Browser Console 首页 |
| `/healthz` | `GET` | 进程健康检查 |
| `/readyz` | `GET` | 依赖准备度 |
| `/livez` | `GET` | 存活检查 |
| `/metrics` | `GET` | 指标快照 |
| `/api/v1/meta` | `GET` | 服务元信息 |
| `/api/v1/memories` | `POST` | 写入文本记忆 |
| `/api/v1/images` | `POST` | 写入图片记忆 |
| `/api/v1/context/search` | `POST` | 搜索上下文 |

### MCP 路由

当配置中启用 `features.enable_mcp = true` 时，服务会暴露：

- `GET /mcp/tools`
- `POST /mcp/tools/call`

当前可用工具：

- `memory.remember`
- `memory.fetch_context`
- `memory.search`
- `memory.publish`

快速查看工具清单：

```bash
curl http://127.0.0.1:8080/mcp/tools
```

## LLM Failover

当前 `models.routing.*` 已支持为每种能力配置：

- `primary`
- `fallbacks`

例如：

```toml
[models.routing.vision]
primary = "gemini_vision"
fallbacks = ["chatgpt_vision", "claude_vision"]
```

当主 LLM 在运行时不可用时，系统会自动切换到下一个可用模型。

当前运行时“不可用”的判断重点包括：

- provider / model 被禁用
- Cloud 模型缺少对应的 `api_key_env`

如果发生切换：

- HTTP 图片写入响应会带 `llm_notice`
- CLI 图片写入 JSON 输出会带 `llm_notice`
- CLI 文本输出会追加 `LLM Notice: ...`

提示文案格式为：

```text
{某}LLM 不可用，已经切换到{新}LLM
```

## 配置说明

默认配置文件：

```text
config/default.toml
```

Docker Compose 使用：

```text
config/docker.toml
```

可通过环境变量覆盖：

```bash
MEAT_MEMORY_CONFIG=config/local.toml cargo run -p memory-app
```

默认关键路径：

| 配置项 | 默认值 |
|---|---|
| HTTP bind | `127.0.0.1:8080` |
| PostgreSQL URL | `postgres://postgres:postgres@127.0.0.1:5433/meat_memory_dev` |
| Markdown root | `./docs` |
| Assets root | `./storage/assets` |

### 推荐的本地覆盖方式

如果你不希望试跑数据直接写进仓库内的 `docs/`，建议创建 `config/local.toml`，例如：

```toml
[logging]
level = "debug"

[postgres]
database_url = "postgres://postgres:postgres@127.0.0.1:5433/meat_memory_dev"

[markdown]
root = "./storage/dev-docs"

[assets]
root = "./storage/dev-assets"

[features]
enable_http = true
enable_mcp = true
```

然后用：

```bash
MEAT_MEMORY_CONFIG=config/local.toml cargo run -p memory-app
```

## 常用脚本

| 命令 | 用途 |
|---|---|
| `./scripts/bootstrap.sh` | 安装 Rust 组件并初始化本地开发环境 |
| `./scripts/verify.sh` | 校验本机工具链、Docker、PG、pgvector 等环境 |
| `./scripts/dev-db-up.sh` | 只启动 `pgvector` |
| `./scripts/dev-db-down.sh` | 只停止 `pgvector` |
| `./scripts/dev-up.sh` | 启动 `pgvector + app + worker` |
| `./scripts/dev-down.sh` | 停止 `pgvector + app + worker` |
| `./scripts/v1-acceptance.sh` | 执行 V1 验收脚本 |

如果你使用 `just`：

```bash
just bootstrap
just verify
just test
just dev-up
just dev-down
```

## 当前测试状态

当前已复核通过：

| 检查项 | 状态 |
|---|---|
| `cargo test -p memory-extract --lib --quiet` | Passed |
| `cargo test -p memory-kernel --test kernel_flow_tests --quiet` | Passed |
| `cargo test -p memory-http --test http_api_tests --quiet` | Passed |
| `cargo test -p memory-mcp --test mcp_tools_tests --quiet` | Passed |
| `cargo test -p memory-cli --test cli_e2e --quiet` | Passed |
| `cargo test --workspace --lib --bins --quiet` | Passed |
| `./scripts/v1-acceptance.sh` | Passed |

最新单测覆盖率快照：

| 指标 | 当前值 |
|---|---:|
| Line Coverage | 95.46% |
| Function Coverage | 91.79% |
| Region Coverage | 87.91% |

详细报告见：

- `test-task.md`
- `target/coverage/unit-pass5/summary.json`
- `target/coverage/unit-pass5/report.txt`

## 仓库结构

```text
crates/
  memory-domain/
  memory-core/
  memory-kernel/
  memory-store/
  memory-store-pg/
  memory-store-md/
  memory-assets/
  memory-index/
  memory-sync/
  memory-policy/
  memory-extract/
  memory-models/
  memory-mcp/
  memory-http/
  memory-worker/
  memory-config/
  memory-observability/
  memory-cli/
  memory-app/
config/
scripts/
docs/
tasks/
tests/
```

## 相关文档

- `docs/meat-memory-scheme-v1.md`
- `docs/meat-memory-scheme-v2.md`
- `docs/api/http-api-v1.md`
- `docs/api/cli-v1.md`
- `docs/api/mcp-tools-v1.md`
- `docs/agent-integration-v1.md`
- `docs/release-notes-v1.md`
- `docs/runbook/local-deploy-v1.md`
- `docs/runbook/cloud-deploy-v1.md`
- `docs/runbook/v1-acceptance.md`
- `tasks/tasklist.md`
- `tasks/task-log.md`

## 当前 V1 边界

`V1` 已完成并封板，当前交付重点是：

- PGSQL + Markdown 双存储
- 本地 / 云独立部署
- HTTP / CLI / MCP 接入
- 多模型抽象
- 文本 + 图片记忆
- 中文优先与系统化中文验收回归
- 知识图谱 baseline

`V1` 明确递延到后续版本或仅做预留的部分：

- 多团队 / 个人隔离与合并
- 中英双语
- 音频 / 视频
- 完整混合部署复制执行

`V1` 封板说明与验收摘要见：

- `docs/release-notes-v1.md`
