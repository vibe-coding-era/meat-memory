# Meat Memory

`Meat Memory` 是一个面向 Agent 和多端接入场景的长期记忆内核，采用 Rust Workspace 实现，支持 `PostgreSQL + Markdown + Assets` 的组合存储，并提供 `CLI`、`HTTP`、`MCP` 三套接入面。

## 你可以先看什么

- 想立刻跑起来：看 [`docs/runbook/usage-guide.md`](docs/runbook/usage-guide.md)
- 想安装或部署：看 [`docs/runbook/install.md`](docs/runbook/install.md)
- 想看当前版本归档：看 [`docs/release-notes-v2_5.md`](docs/release-notes-v2_5.md)
- 想理解整体实现：看 [`docs/architecture/system-design.md`](docs/architecture/system-design.md)
- 想找所有文档入口：看 [`docs/README.md`](docs/README.md)
- 想查命令和协议：看 [`docs/api/README.md`](docs/api/README.md)

## 当前能力

| 能力域 | 状态 |
| --- | --- |
| 存储 | PostgreSQL 主存 + Markdown 投影 + 本地资产目录 |
| 接入层 | CLI / HTTP API / MCP / Browser Console |
| 模型集成 | 多 provider 路由、能力分组、fallback 机制 |
| 记忆类型 | 文本、图片、短期上下文、项目文档同步 |
| 运维 | Docker Compose、本地开发脚本、健康检查、测试报告 |
| 测试 | unit / integration / e2e / perf / security 分层 |

## 仓库结构

```text
.
├── config/          运行配置与配置说明
├── crates/          Rust workspace 各业务 crate
├── docs/            使用说明、架构设计、脚本、API 与运维文档
├── infra/           Docker / Helm 等基础设施资产
├── migrations/      PostgreSQL schema 迁移
└── tests/           测试入口与测试报告目录说明
```

`crates/` 中的职责分层可概括为：

- `memory-app` / `memory-worker`：进程入口
- `memory-cli`：命令行和初始化向导
- `memory-http` / `memory-mcp`：协议适配层
- `memory-kernel`：核心编排与业务流程
- `memory-store-*`：Markdown / PostgreSQL 存储实现
- `memory-domain` / `memory-core` / `memory-policy` / `memory-sync`：领域模型和基础能力

## 快速开始

### 方案 A：Docker Compose 试跑

```bash
cp .env.example .env
./docs/scripts/dev-up.sh
curl http://127.0.0.1:8080/healthz
curl http://127.0.0.1:8080/api/v1/meta
```

停止环境：

```bash
./docs/scripts/dev-down.sh
```

### 方案 B：本地开发运行

```bash
cp .env.example .env
./docs/scripts/dev-db-up.sh
./docs/scripts/bootstrap.sh
./docs/scripts/verify.sh
./docs/scripts/test-required.sh
cargo run -p memory-app
```

也可以通过 CLI 启动：

```bash
cargo run -p memory-cli -- serve --bind 127.0.0.1:8080
```

### 方案 C：安装 CLI 到本机

```bash
cargo install --path crates/memory-cli --locked
memory-cli --help
memory-cli config check
memory-cli mcp info
memory-cli skills export --target all --output-dir ./dist/agent-skills --force
```

更多安装、打包和部署方式见 [`docs/runbook/install.md`](docs/runbook/install.md)。

## 常用命令

```bash
cargo run -p memory-cli -- config check
cargo run -p memory-cli -- config check --database
cargo run -p memory-cli -- mcp info
cargo run -p memory-cli -- tui init
cargo run -p memory-cli -- skills export --target all --output-dir ./dist/agent-skills --force
```

写入与检索示例：

```bash
cargo run -p memory-cli -- remember \
  --scope-id scp_cli_demo \
  --title "评审规则" \
  --body "代码评审先列风险，再列摘要。" \
  --memory-kind preference \
  --json

cargo run -p memory-cli -- search "评审 风险" \
  --scope-id scp_cli_demo \
  --limit 5 \
  --json
```

## 服务入口

默认地址为 `http://127.0.0.1:8080`。

常用路由：

| 路由 | 方法 | 用途 |
| --- | --- | --- |
| `/` | `GET` | Browser Console |
| `/healthz` | `GET` | 健康检查 |
| `/readyz` | `GET` | 依赖准备度 |
| `/metrics` | `GET` | 指标 |
| `/api/v1/meta` | `GET` | 服务元信息 |
| `/api/v1/memories` | `POST` | 写入文本记忆 |
| `/api/v1/images` | `POST` | 写入图片记忆 |
| `/api/v1/context/search` | `POST` | 检索上下文 |
| `/mcp/tools` | `GET` | MCP 工具发现 |

## 文档导航

- [`docs/runbook/usage-guide.md`](docs/runbook/usage-guide.md)：完整使用说明
- [`docs/runbook/install.md`](docs/runbook/install.md)：安装、打包与部署入口
- [`docs/release-notes-v2_5.md`](docs/release-notes-v2_5.md)：V2.5 版本归档
- [`docs/architecture/system-design.md`](docs/architecture/system-design.md)：系统架构设计
- [`docs/runbook/README.md`](docs/runbook/README.md)：部署与验收入口
- [`docs/agent-skills/README.md`](docs/agent-skills/README.md)：Agent Skill 模板和导出方式
- [`docs/scripts/README.md`](docs/scripts/README.md)：脚本入口导航
- [`crates/README.md`](crates/README.md)：workspace 模块职责导航
- [`migrations/README.md`](migrations/README.md)：数据库迁移说明
- [`infra/README.md`](infra/README.md)：基础设施目录说明
- [`config/README.md`](config/README.md)：配置组织方式
- [`storage/README.md`](storage/README.md)：本地运行时目录约定
- [`tests/README.md`](tests/README.md)：测试布局和报告位置
