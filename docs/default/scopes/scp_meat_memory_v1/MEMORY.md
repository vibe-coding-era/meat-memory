# Scope Memory Rollup

<!-- memory-entry:start mem_01KNVETF7A7WFK14X774P4MWPW -->
---
id: mem_01KNVETF7A7WFK14X774P4MWPW
kind: memory
tenant: default
scope: scp_meat_memory_v1
memory_kind: summary
title: file::README.md
status: active
visibility: private
sensitivity: internal
created_at: 2026-04-10T10:25:52.362286Z
updated_at: 2026-04-10T10:25:52.362287Z
source_refs: []
evidence: []
entities: []
tags: []
scores:
  confidence: 0.6
  importance: 0.5
  stability: 0.5
  freshness: 1.0
evidence_count: 1
---
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

## 快速开始

如果你只是想先把服务跑起来，推荐直接走 Docker Compose。

### 方案 A：最快试跑

1. 复制环境变量文件。

```bash
cp .env.example .env
```

2. 启动本地完整栈。

```bash
./docs/scripts/dev-up.sh
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
./docs/scripts/dev-down.sh
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
./docs/scripts/dev-db-up.sh
```

3. 安装 Rust 组件并初始化本地开发环境。

```bash
./docs/scripts/bootstrap.sh
```

4. 运行环境校验。

```bash
./docs/scripts/verify.sh
```

注意：

- `verify.sh` 会检查 `pgvector` 是否可连通。
- 所以第一次试跑时，应该先执行 `./docs/scripts/dev-db-up.sh`，再执行 `./docs/scripts/verify.sh`。

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
./docs/scripts/dev-db-down.sh
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
| `./docs/scripts/bootstrap.sh` | 安装 Rust 组件并初始化本地开发环境 |
| `./docs/scripts/verify.sh` | 校验本机工具链、Docker、PG、pgvector 等环境 |
| `./docs/scripts/dev-db-up.sh` | 只启动 `pgvector` |
| `./docs/scripts/dev-db-down.sh` | 只停止 `pgvector` |
| `./docs/scripts/dev-up.sh` | 启动 `pgvector + app + worker` |
| `./docs/scripts/dev-down.sh` | 停止 `pgvector + app + worker` |
| `./docs/scripts/v1-acceptance.sh` | 执行 V1 验收脚本 |

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
| `./docs/scripts/v1-acceptance.sh` | Passed |

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
docs/scripts/
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
<!-- memory-entry:end mem_01KNVETF7A7WFK14X774P4MWPW -->

<!-- memory-entry:start mem_01KNVETF82GXBKM6PEX0AZN3H0 -->
---
id: mem_01KNVETF82GXBKM6PEX0AZN3H0
kind: memory
tenant: default
scope: scp_meat_memory_v1
memory_kind: summary
title: file::tasks/tasklist.md
status: active
visibility: private
sensitivity: internal
created_at: 2026-04-10T10:25:52.386917Z
updated_at: 2026-04-10T10:25:52.386919Z
source_refs: []
evidence: []
entities: []
tags: []
scores:
  confidence: 0.6
  importance: 0.5
  stability: 0.5
  freshness: 1.0
evidence_count: 1
---
# Meat Memory 版本化 TaskList

更新时间：2026-04-10

## 规划原则

- 本文件从“原子级全量拆解”切换为“版本化交付清单”。
- 从现在开始，`V1 / V2 / V3` 是范围管理的唯一主视图。
- 历史执行记录仍保留在 `tasks/task-log.md`，其中旧的 `MM-*` 编号继续有效。
- 新任务优先使用 `V1-*`、`V2-*`、`V3-*` 编号。
- 本文件只描述版本目标、交付范围、任务状态和建议顺序，不等同于实现日志。

## 当前快照

- 已完成底座：Rust workspace、PGSQL + Markdown 双存储、kernel `remember/search/publish`、知识图谱抽取、HTTP/CLI/MCP 接入层、基础 observability、sync oplog/merge baseline、V1 多模型能力抽象与 provider/model/route registry、V1 图片资产存储与寻址基线、V1 `remember_image` 写入链路、V1 最小图片理解链路、Agent 接入文档、Docker/Helm 部署骨架、V1 验收脚本与文档包、中文系统化验收语料、Browser Console / failover 回归入口，以及 V1 release notes / 封板说明。
- 当前复核状态：已确认 `memory-extract`、`memory-kernel`、`memory-http`、`memory-mcp`、`memory-cli` 定向回归，`cargo test --workspace --lib --bins --quiet`、`./docs/scripts/v1-acceptance.sh`、`docker compose config --quiet`、`helm lint infra/helm/meat-memory` 通过，V1 已完成。
- 最新单测覆盖率快照：Line `95.46%`、Function `91.79%`、Region `87.91%`，产物位于 `target/coverage/unit-pass5/`。
- 当前最重要的未完成范围：V2 的团队隔离/多语言规划准备。

## 版本范围总览

| 版本 | 目标范围 | 明确不做 |
|---|---|---|
| V1 | PGSQL + Markdown、云或本地独立部署、为混合部署预留接口、支持 Codex/Claude Code/TRAE/Qoder 与 OpenClaw/CoWork/QoderWork、跨大模型抽象、文本+图片、中文优先、知识图谱 | 不做多团队/个人隔离与合并，不做英文，不做音频/视频 |
| V2 | 多团队/个人隔离与合并、多语言扩展到中/英 | 不做音频/视频 |
| V3 | 补齐全多模态，完成音频/视频 | 无 |

## V1 交付清单

### V1 版本验收标准

- 支持 PostgreSQL 作为结构化主存，Markdown 作为 projection 和本地侧车。
- 支持本地独立部署和云端独立部署。
- 为混合部署保留 sync/oplog/merge 抽象接口，但不要求 V1 完成完整混合复制执行。
- 支持文本记忆和图片记忆。
- 支持中文优先的记忆抽取、检索和知识图谱。
- 支持 HTTP、CLI、MCP 三类 Agent 接入面。
- 支持跨大模型 provider 抽象层，但不要求一次性完成全部模型深度适配。

### V1 任务表

| ID | 任务 | 状态 | 说明 |
|---|---|---|---|
| `V1-STO-001` | PostgreSQL 主存链路 | done | 已完成 schema、写入、检索、发布更新和集成测试 |
| `V1-STO-002` | Markdown projection 链路 | done | 已完成 frontmatter、rollup、parser、roundtrip 测试 |
| `V1-KER-001` | `remember_text` / `search_context` / `publish_memory` | done | 已完成 kernel 主链路和 remember→search→publish 测试 |
| `V1-KG-001` | 知识图谱基线：memory/entity/relation 抽取 | done | 已完成 distill、entity、relation、graph context |
| `V1-API-001` | HTTP API：`/memories`、`/context/search`、健康检查、metrics | done | 已完成 HTTP handler 与集成测试 |
| `V1-API-002` | CLI：`remember`、`search`、`serve` | done | 已完成真实命令与 smoke |
| `V1-API-003` | MCP：`remember`、`search`、`fetch_context`、`publish` | done | 已完成 dispatcher、stdio/http transport skeleton 与集成测试 |
| `V1-API-004` | Agent 接入说明，覆盖 Codex / Claude Code / TRAE / Qoder / OpenClaw / CoWork / QoderWork | done | 已完成 Agent 接入指南，并让 `memory-app` / `memory-cli serve` 在启用配置时真实挂出 MCP HTTP 路由 |
| `V1-MOD-001` | 多模型能力抽象：reasoning / extraction / vision / embedding | done | 已完成 trait、request/response、descriptor、route/fallback registry |
| `V1-MOD-002` | Provider registry 与配置层，兼容 Gemini / Claude / ChatGPT / 千问 / 豆包 / Minimax / GLM | done | 已完成 provider catalog、默认配置、启动期 registry 校验与测试 |
| `V1-MUL-001` | 图片资产存储与寻址 | done | 已完成 sha256 寻址、本地文件存储、分类目录、metadata 与回读测试 |
| `V1-MUL-002` | 图片记忆写入链路，等价于 `remember_image` | done | 已完成 kernel/HTTP/CLI 图片写入、资产落盘与集成测试 |
| `V1-MUL-003` | 图片理解最小能力：OCR / caption / vision extraction 至少一条 | done | 已完成本地 image profile 解析、vision gateway、自动 caption 与派生文本写回 |
| `V1-ZH-001` | 中文优先抽取、检索、图谱样例与测试语料 | done | 已补齐 `tests/integration/v1-zh-acceptance.md`，并覆盖 kernel / HTTP / MCP / CLI 中文回归、Browser Console 首页与图片 failover 提示回归 |
| `V1-DEP-001` | 本地独立部署包：compose + app + worker + pgvector | done | 已完成 compose 卷持久化、`memory-worker` 真实入口、本地 runbook、`docker compose config` 校验 |
| `V1-DEP-002` | 云端独立部署包：Docker/Helm/最小发布闭环 | done | 已完成 Dockerfile 修正、`.dockerignore`、`config/cloud.example.toml`、Helm chart、`helm lint` 与镜像构建验证 |
| `V1-SYN-001` | 为混合部署预留 sync/oplog/merge 抽象接口 | done | 已有 `OplogEntry`、cursor/batch、append、merge、内存 replication engine |
| `V1-OBS-001` | V1 可观测性：结构化日志、延迟/命中率指标、health/ready/live/metrics | done | 已完成第一阶段 observability |
| `V1-QA-001` | V1 端到端验收：HTTP + CLI + MCP + 文本 + 图片 + provider mock | done | 已完成 CLI E2E、HTTP/MCP 集成测试、`docs/scripts/v1-acceptance.sh` 与 provider mock 路由命中校验 |
| `V1-DOC-001` | V1 文档包：安装、部署、Agent 接入、API/MCP/CLI 使用说明 | done | 已完成 Agent/HTTP/MCP/CLI/本地部署/云部署/验收文档收口 |
| `V1-REL-003` | V1 版本封板、release notes 与交付边界收口 | done | 已补齐 `docs/release-notes-v1.md`、更新 `CHANGELOG.md` / `README.md` / `tasks/*`，并将仓库状态收口到 “V1 已完成” |

### V1 剩余建议执行顺序

- 无，`V1` 已完成。

## V2 交付清单

### V2 版本验收标准

- 支持多团队 / 个人隔离与合并。
- 支持中英双语，而不是中文优先的单语形态。
- 建立更完整的共享、发布、合并、冲突处理流程。

### V2 任务表

| ID | 任务 | 状态 | 说明 |
|---|---|---|---|
| `V2-SCP-001` | 多团队 / 个人 scope 模型正式化 | todo | 将当前 scope 基线扩展到团队/个人隔离真实规则 |
| `V2-SCP-002` | 发布、review、共享、隔离策略扩展 | todo | 需要补 policy/redaction/review 工作流 |
| `V2-MRG-001` | 多节点 merge 策略与冲突对象保留 | todo | 在现有 `merge_ops` 基线之上扩展 |
| `V2-SYN-001` | 持久化 replication engine 与真实 pull/apply | todo | 当前只有内存版 replication engine |
| `V2-SYN-002` | 团队级同步、跨节点回放、审计链路 | todo | 需要和 worker、projection refresh 联动 |
| `V2-LNG-001` | 英文抽取、检索、图谱支持 | todo | V1 只承诺中文优先 |
| `V2-LNG-002` | 中英双语测试集、回归测试、文档 | todo | V2 的语言验收入口 |

### V2 建议执行顺序

1. `V2-SCP-001`
2. `V2-SCP-002`
3. `V2-SYN-001`
4. `V2-MRG-001`
5. `V2-LNG-001`
6. `V2-LNG-002`

## V3 交付清单

### V3 版本验收标准

- 补齐音频和视频链路。
- 多模态从“文本 + 图片”扩展为“文本 + 图片 + 音频 + 视频”。
- 检索、抽取、证据链和 Agent 工具面都覆盖全多模态。

### V3 任务表

| ID | 任务 | 状态 | 说明 |
|---|---|---|---|
| `V3-MM-001` | 音频 ingest、资产存储、转写入口 | todo | V1/V2 都不包含 |
| `V3-MM-002` | 视频 ingest、关键帧与时间轴 evidence | todo | V1/V2 都不包含 |
| `V3-MM-003` | 音频/视频 extraction pipeline | todo | 需要 speech-to-text / caption / multimodal extraction |
| `V3-MM-004` | 全多模态检索与 late fusion | todo | 统一文本、图片、音频、视频召回 |
| `V3-MM-005` | MCP / HTTP / CLI 的全多模态工具面 | todo | 在现有接入层上扩展全多模态能力 |

### V3 建议执行顺序

1. `V3-MM-001`
2. `V3-MM-002`
3. `V3-MM-003`
4. `V3-MM-004`
5. `V3-MM-005`

## 历史任务映射

本节用于保持和 `tasks/task-log.md`、`tasks/project-index.md` 的连续性。

| 旧任务族 | 新版本归属 |
|---|---|
| `MM-PG-*`、`MM-MD-*` | `V1-STO-*` |
| `MM-KER-001/002/004/005/006` | `V1-KER-001` |
| `MM-EXT-*` | `V1-KG-001` |
| `MM-HTTP-*` | `V1-API-001` |
| `MM-CLI-*` | `V1-API-002` |
| `MM-MCP-*` | `V1-API-003` |
| `MM-OBS-001/002/003/006` | `V1-OBS-001` |
| `MM-SYNC-001/002/003/006` | `V1-SYN-001` |
| `MM-KER-003` | `V1-MUL-002` 的上位任务来源 |
| `MM-SYNC-004/005` | `V1-DEP-*` 与后续混合部署准备任务 |

## 当前下一步

- 第一优先级：准备 V2 的 scope/多语言工作
<!-- memory-entry:end mem_01KNVETF82GXBKM6PEX0AZN3H0 -->

<!-- memory-entry:start mem_01KNVETF8408HH1WWC41GFKV2K -->
---
id: mem_01KNVETF8408HH1WWC41GFKV2K
kind: memory
tenant: default
scope: scp_meat_memory_v1
memory_kind: summary
title: file::tasks/project-index.md
status: active
visibility: private
sensitivity: internal
created_at: 2026-04-10T10:25:52.388556Z
updated_at: 2026-04-10T10:25:52.388558Z
source_refs: []
evidence: []
entities: []
tags: []
scores:
  confidence: 0.6
  importance: 0.5
  stability: 0.5
  freshness: 1.0
evidence_count: 1
---
# Meat Memory 项目索引

更新时间：2026-04-10

索引范围：`/Users/Rou/dev_projects/meat-memory`

## 1. 当前目录结构

```text
.
├── .cargo/
├── .env.example
├── .githooks/
├── .github/
├── config/
├── crates/
├── docs/
│   ├── meat-memory-prompt.md
│   ├── release-notes-v1.md
│   ├── meat-memory-scheme-v1.md
│   └── meat-memory-scheme-v2.md
├── infra/
├── docs/scripts/
└── tasks/
    ├── project-index.md
    ├── task-log.md
    └── tasklist.md
├── tests/
├── Cargo.toml
├── CHANGELOG.md
├── Dockerfile
├── LICENSE
├── README.md
├── compose.yaml
├── justfile
└── rust-toolchain.toml
```

说明：

- 当前仓库已经从“纯文档仓库”推进到“工程基线 + 存储层 + kernel 主链路 + 基础可观测性 + V1 模型网关与图片理解基线 + 部署/文档/验收收口 + 中文系统化回归补齐 + V1 封板完成”。
- 已具备 Rust workspace、核心领域模型、PG/Markdown 存储层、`memory-kernel` remember/search/publish 编排、entity/relation 抽取、`memory-models` capability traits + provider/model/route registry + 本地 vision gateway、HTTP remember/search/health/metrics 路由、CLI remember/search/serve 入口、MCP remember/search/fetch_context/publish tool 层、内存版 sync oplog/merge baseline、基础配置、脚本、CI skeleton、Dockerfile、compose、本地 pgvector 开发库、Helm chart、V1 文档包、V1 验收脚本与 Git hooks。
- 当前最重要的有效输入变成了三类：`docs/` 下的设计文档、`tasks/` 下的执行文档、`crates/` 下的真实工程实现

## 2. 文件角色索引

### 2.1 `docs/`

`docs/meat-memory-prompt.md`

- 角色：原始需求输入
- 用途：记录产品目标、兼容范围、技术方向和约束
- 当前价值：作为所有方案与任务拆解的根需求源

`docs/meat-memory-scheme-v1.md`

- 角色：第一版总体方案
- 用途：定义长期 Memory 的总体方向、关键能力和边界
- 当前价值：作为高层架构与范围界定参考

`docs/meat-memory-scheme-v2.md`

- 角色：细化后的架构设计主文档
- 用途：定义对象模型、写入/检索链路、存储分层、同步、协议、Rust workspace 设计
- 当前价值：后续实现的主依据

`docs/release-notes-v1.md`

- 角色：V1 封板说明
- 用途：沉淀 V1 交付边界、验收摘要、递延范围和操作入口
- 当前价值：作为 V1 对外交付与内部交接的统一摘要

### 2.2 `tasks/`

`tasks/tasklist.md`

- 角色：原子任务总清单
- 用途：从环境安装、仓库脚手架、内核实现到部署上线的执行路线图
- 当前价值：执行入口文档

`tasks/task-log.md`

- 角色：结构化执行日志
- 用途：记录索引、任务完成、决策、阻塞和下一步动作
- 当前价值：执行过程上下文主日志

`tasks/project-index.md`

- 角色：项目现状索引
- 用途：提供快速上下文、目录盘点、状态判断和开工入口
- 当前价值：执行前阅读入口

### 2.3 工程基线

`Cargo.toml`

- 角色：Rust workspace 根清单
- 用途：统一 members、共享依赖和 lint 策略
- 当前价值：workspace 已可 `cargo check`

`rust-toolchain.toml`

- 角色：Rust 工具链 pin
- 用途：统一团队编译版本与组件
- 当前价值：已固定到 `1.85.0`

`config/default.toml`

- 角色：默认运行配置
- 用途：定义 bind、logging、markdown、postgres、assets、models、sync、features
- 当前价值：`memory-app`、`memory-cli`、`memory-worker` 已从此文件读取并校验模型 registry

`docs/scripts/`

- 角色：本地开发与验证入口
- 用途：环境验证、bootstrap、本地 pgvector 库启动与关闭、V1 验收执行
- 当前价值：`verify.sh`、`bootstrap.sh`、`dev-db-up.sh`、`dev-db-down.sh`、`v1-acceptance.sh` 已可用

`compose.yaml`

- 角色：本地容器编排
- 用途：启动项目专用 `pgvector`、`app`、`worker` 本地栈
- 当前价值：已补齐 Markdown/assets 持久卷，`docker compose config --quiet` 通过

`Dockerfile`

- 角色：应用运行镜像骨架
- 用途：提供本地容器运行和云部署镜像基础
- 当前价值：已修正 `memory-worker` 二进制复制链路，并通过 `docker build -t meat-memory:local-check .`

`infra/helm/meat-memory/`

- 角色：V1 云端独立部署包
- 用途：提供 app/service/pvc/config/secret 的最小 Helm 发布闭环
- 当前价值：`helm lint infra/helm/meat-memory` 已通过

## 3. 当前实现状态

当前状态判断：

- 需求已形成
- 高层方案已形成
- 细化设计已形成
- 执行任务清单已形成
- 执行日志机制已形成
- 工程仓库已初始化
- 核心领域模型已实现并有单测
- PostgreSQL store 已实现并有集成测试
- Markdown store 已实现并有 roundtrip 测试
- memory-kernel 已实现最小 dual-write remember/search 编排
- memory-policy 已提供最小 write policy allow/review/deny 规则
- memory-extract 已提供 distill/entity/relation 抽取与独立测试
- memory-kernel 已实现 publish_memory 与 PG+Markdown remember→search→publish 集成测试
- HTTP `POST /api/v1/memories`、`POST /api/v1/images` 与 `POST /api/v1/context/search` 已可用
- HTTP `GET /` Browser Console 首页已可用
- HTTP `/healthz`、`/readyz`、`/livez`、`/metrics` 已可用
- `memory-cli` 已支持 `remember`、`search`、`serve`
- `memory-mcp` 已支持 tool listing、stdio message handler、HTTP `/mcp/tools`/`/mcp/tools/call`，以及 `memory.remember`、`memory.search`、`memory.fetch_context`、`memory.publish`
- `memory-app` 与 `memory-cli serve` 在开启 `enable_mcp` 时已真实挂出 MCP HTTP 路由
- `memory-models` 已支持 reasoning / extraction / vision / embedding 抽象、provider/model descriptor、fallback route registry
- `memory-models` 已支持本地 image profile 解析、vision gateway 和中文优先 caption 生成
- `memory-config` 已接入 Gemini / Claude / ChatGPT / 千问 / 豆包 / Minimax / GLM 的 provider catalog 与默认路由配置
- `memory-assets` 已支持 sha256 寻址、本地文件存储、分类目录、asset metadata 与回读测试
- `memory-kernel` 已支持 `remember_image`
- HTTP 已支持 `POST /api/v1/images`，CLI 已支持 `remember-image`
- 图片写入时已自动生成最小 vision caption 与结构化派生文本，并写回 Artifact/Memory
- `memory-sync` 已支持 `OplogEntry`/`SyncCursor`/`SyncBatch`/`ApplyBatchResult`、`append_oplog_entry`、`merge_ops` 与内存版 replication engine
- memory-observability 已提供结构化日志字段约定与检索/写入指标快照
- 最小 app/cli/worker 入口已可编译、运行并在启动期校验模型路由配置
- docker compose 本地栈已具备 app/worker/pgvector 拓扑、持久卷和配置校验
- `.env.example`、`justfile`、pre-commit/commit-msg hooks 已补齐
- 本地 pgvector 目标库已可用
- V1 用户文档包已补齐：Agent 接入、HTTP/MCP/CLI、local/cloud runbook、acceptance
- V1 验收脚本 `./docs/scripts/v1-acceptance.sh` 已补齐并通过
- V1 中文系统化验收语料已补齐：`tests/integration/v1-zh-acceptance.md`
- 中文 remember/search/publish 回归已覆盖 kernel / HTTP / MCP / CLI
- Browser Console 首页与图片 failover `llm_notice` 已纳入回归入口
- V1 release notes、CHANGELOG 与 README 边界说明已补齐

换句话说，当前项目已经从“想清楚”和“拆任务”的阶段，进入了“V1 已完成并可交付，后续进入 V2 准备”的阶段。

## 4. 当前剩余的关键工作

以下工作目前仍需继续推进：

- V2 范围内的多团队/个人隔离与中英双语扩展

这意味着项目已经从“收口 V1 版本边界”推进到“准备 V2”的阶段。

## 5. 推荐执行起点

建议执行顺序：

1. 准备 V2 的 scope 与多语言扩展

## 6. 建议的第一批可执行任务

最先启动的任务建议是：

- V2 规划准备

当前项目已经处在“V1 已完成、可交付、可试跑”的状态。

## 7. 当前风险与注意事项

- 当前图片 caption 已可自动生成，但尚未接入真实外部视觉 SDK，V1 仍属于“本地最小可运行 + 可切真实 SDK”的形态
- `tasklist.md` 已切到版本视图，后续执行与汇报应优先使用 `V1-* / V2-* / V3-*` 编号
- 若后续目录结构发生变化，应同步更新本文件和 `task-log.md`
- `docs/meat-memory-scheme-v2.md` 应继续作为主设计依据，避免实现过程漂移回旧的 `MM-*` 粒度规划
- `.cargo/config.toml` 已从“全局固定 `/usr/bin/clang`”收敛到“仅 Apple target 固定 clang”，避免 Linux 容器构建失败

## 8. 下一步建议

建议下一步直接扩展当前可运行闭环：

1. 进入 V2 的 scope/多语言规划准备。
<!-- memory-entry:end mem_01KNVETF8408HH1WWC41GFKV2K -->

<!-- memory-entry:start mem_01KNVETF89264Q9P9M27KT59KD -->
---
id: mem_01KNVETF89264Q9P9M27KT59KD
kind: memory
tenant: default
scope: scp_meat_memory_v1
memory_kind: summary
title: file::docs/release-notes-v1.md
status: active
visibility: private
sensitivity: internal
created_at: 2026-04-10T10:25:52.393927Z
updated_at: 2026-04-10T10:25:52.393928Z
source_refs: []
evidence: []
entities: []
tags: []
scores:
  confidence: 0.6
  importance: 0.5
  stability: 0.5
  freshness: 1.0
evidence_count: 1
---
# V1 Release Notes

更新时间：2026-04-10

## 1. 发布信息

| 项目 | 内容 |
| --- | --- |
| 版本 | `0.1.0` |
| 发布阶段 | `V1` 封板版本 |
| 封板日期 | `2026-04-09` |
| 目标定位 | 纯自研、可自托管的长期 Memory 内核 |
| 技术栈 | Rust workspace + PostgreSQL + Markdown projection |
| 部署口径 | 本地独立部署、云端独立部署，混合部署预留接口 |

## 2. V1 交付范围

| 范围 | V1 结果 |
| --- | --- |
| 结构化主存 | 已支持 PostgreSQL 写入、检索、发布更新 |
| 本地投影 | 已支持 Markdown projection、rollup、roundtrip |
| 核心链路 | 已支持 `remember_text`、`search_context`、`publish_memory` |
| 知识图谱 | 已支持 entity / relation 抽取与 graph context baseline |
| Agent 接入 | 已支持 HTTP / CLI / MCP |
| 模型抽象 | 已支持 reasoning / extraction / vision / embedding 能力抽象 |
| 模型目录 | 已兼容 Gemini / Claude / ChatGPT / 千问 / 豆包 / MiniMax / GLM |
| Failover | 已支持 `primary + fallbacks` 自动切换与 `llm_notice` 提示 |
| 多模态 | V1 已支持文本 + 图片 |
| 中文优先 | 已支持中文抽取、检索、图谱样例、中文 caption |
| 部署 | 已支持 Compose 本地栈、Docker 镜像、Helm chart |
| 同步抽象 | 已提供 `sync/oplog/merge` baseline，为混合部署预留接口 |

## 3. 验收入口

| 入口 | 用途 |
| --- | --- |
| `cargo test --workspace --lib --bins --quiet` | 工作区主回归 |
| `./docs/scripts/v1-acceptance.sh` | V1 自动化验收入口 |
| `docker compose config --quiet` | 本地部署编排校验 |
| `helm lint infra/helm/meat-memory` | 云部署 chart 校验 |

## 4. 关键回归摘要

| 模块 | 已覆盖内容 |
| --- | --- |
| `memory-extract` | 中文关系抽取与图谱样例 |
| `memory-kernel` | 中文 remember / search / publish |
| `memory-http` | 中文文本流、Browser Console 首页 `/`、图片 failover `llm_notice` |
| `memory-mcp` | 中文 remember / search / publish 工具调用 |
| `memory-cli` | 中文 remember / search、图片 failover JSON 输出 |
| `tests/integration/v1-zh-acceptance.md` | 中文系统化验收语料入口 |

## 5. 使用入口

| 入口 | 说明 |
| --- | --- |
| [README.md](/Users/Rou/dev_projects/meat-memory/README.md) | 本地试跑、接口说明、V1 边界 |
| [docs/agent-integration-v1.md](/Users/Rou/dev_projects/meat-memory/docs/agent-integration-v1.md) | Agent 接入说明 |
| [docs/api/http-api-v1.md](/Users/Rou/dev_projects/meat-memory/docs/api/http-api-v1.md) | HTTP API |
| [docs/api/cli-v1.md](/Users/Rou/dev_projects/meat-memory/docs/api/cli-v1.md) | CLI 用法 |
| [docs/api/mcp-tools-v1.md](/Users/Rou/dev_projects/meat-memory/docs/api/mcp-tools-v1.md) | MCP 工具面 |
| [docs/runbook/local-deploy-v1.md](/Users/Rou/dev_projects/meat-memory/docs/runbook/local-deploy-v1.md) | 本地部署 |
| [docs/runbook/cloud-deploy-v1.md](/Users/Rou/dev_projects/meat-memory/docs/runbook/cloud-deploy-v1.md) | 云端部署 |
| [docs/runbook/v1-acceptance.md](/Users/Rou/dev_projects/meat-memory/docs/runbook/v1-acceptance.md) | V1 验收口径 |

## 6. 明确不在 V1 的范围

| 范围 | 归属版本 |
| --- | --- |
| 多团队 / 个人隔离与合并 | `V2` |
| 中英双语 | `V2` |
| 音频 / 视频 | `V3` |
| 完整混合部署复制执行 | `V2+` |

## 7. 当前结论

`V1` 已完成并封板。当前仓库已经具备：

- 可运行的文本 + 图片长期 Memory 最小闭环
- 面向 Agent 的 HTTP / CLI / MCP 接入面
- 中文优先的抽取、检索与图谱回归入口
- 本地独立部署与云端独立部署的最小交付包

后续主线转入 `V2`：团队/个人隔离、发布共享策略扩展，以及中英双语支持。
<!-- memory-entry:end mem_01KNVETF89264Q9P9M27KT59KD -->

<!-- memory-entry:start mem_01KNVEV238MQD0CQKR5E7ASS5B -->
---
id: mem_01KNVEV238MQD0CQKR5E7ASS5B
kind: memory
tenant: default
scope: scp_meat_memory_v1
memory_kind: summary
title: 项目现状摘要
status: active
visibility: private
sensitivity: internal
created_at: 2026-04-10T10:26:11.68831Z
updated_at: 2026-04-10T10:26:11.688311Z
source_refs: []
evidence: []
entities: []
tags: []
scores:
  confidence: 0.6
  importance: 0.5
  stability: 0.5
  freshness: 1.0
evidence_count: 1
---
Meat Memory 仓库当前已完成 V1 封板，核心实现包含 Rust workspace、PostgreSQL 与 Markdown 双存储、memory-kernel remember/search/publish 主链路、HTTP/CLI/MCP 接入、文本加图片 memory、知识图谱抽取、Browser Console 工作台，以及本地与云端部署骨架。当前下一步聚焦 V2 的 scope 隔离、多团队协作与中英双语扩展。
<!-- memory-entry:end mem_01KNVEV238MQD0CQKR5E7ASS5B -->
