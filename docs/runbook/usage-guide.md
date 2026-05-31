# Meat Memory 使用说明

本文档面向已经克隆源码仓库的开发者、运维同学和 Agent 集成方，目标是回答三件事：

- 这个项目怎么启动
- 常见场景应该用哪条命令
- 出问题时应该去哪里排查

如果你的目标是新用户安装，不要运行本文中的 `./docs/scripts/*` 源码开发脚本；直接使用一行 release 安装命令：

```bash
curl -fsSL https://raw.githubusercontent.com/vibe-coding-era/meat-memory/release-V1/deploy/release-V1/install.sh | bash -s -- --skip-tui --verify-write
```

如果你的目标是选择安装方式、打包方式或部署形态，建议先看 [`install.md`](install.md)。

## 1. 运行前准备

基础依赖：

- Rust `1.85+`
- Docker 与 Docker Compose
- 可访问的 PostgreSQL 17 + `pgvector`
- 一份可用的环境变量文件：`.env`

初始化建议顺序：

```bash
cp .env.example .env
./docs/scripts/bootstrap.sh
./docs/scripts/verify.sh
```

如果只是本地开发，不想先完整起容器，可以只拉起数据库：

```bash
./docs/scripts/dev-db-up.sh
```

## 2. 两种启动方式

### 2.1 最快试跑

适合第一次验收仓库、验证 HTTP 与 MCP 接口是否可用。

```bash
cp .env.example .env
./docs/scripts/dev-up.sh
curl http://127.0.0.1:8080/healthz
curl http://127.0.0.1:8080/api/v1/meta
curl http://127.0.0.1:8080/mcp/tools
```

停止：

```bash
./docs/scripts/dev-down.sh
```

### 2.2 本地开发模式

适合调试 Rust 代码、单步查看日志或只重启某个二进制。

```bash
cp .env.example .env
./docs/scripts/dev-db-up.sh
./docs/scripts/bootstrap.sh
./docs/scripts/verify.sh
./docs/scripts/test-required.sh
cargo run -p memory-app
```

也可以用 CLI 启服务：

```bash
cargo run -p memory-cli -- serve --bind 127.0.0.1:8080
```

## 3. 初始化与配置检查

默认主配置文件是 `config/app.toml`，建议把本地私有差异写到 `config/app.local.toml`。

推荐命令：

```bash
cargo run -p memory-cli -- config show
cargo run -p memory-cli -- config check
cargo run -p memory-cli -- config check --database
cargo run -p memory-cli -- tui init --interactive --write-config config/app.local.toml
```

如果要使用本地覆盖配置：

```bash
MEAT_MEMORY_CONFIG=config/app.local.toml cargo run -p memory-cli -- config check
```

如果你希望先把 CLI 安装到本机，再执行这些检查，也可以直接使用：

```bash
cargo install --path crates/memory-cli --locked
memory-cli --help
memory-cli config check
memory-cli mcp info
memory-cli skills export --target all --output-dir ./dist/agent-skills --force
```

适用场景：

- 想验证 `cargo install` 路径是否可用
- 想把 `memory-cli` 暴露给本地 shell 或其他脚本
- 不想每次都写 `cargo run -p memory-cli -- ...`

## 4. 常见使用场景

### 4.1 写入一条文本记忆

```bash
cargo run -p memory-cli -- remember \
  --scope-id scp_demo \
  --title "发布规则" \
  --body "生产变更必须先通过回归测试。" \
  --memory-kind procedure \
  --json
```

### 4.2 搜索记忆

```bash
cargo run -p memory-cli -- search "发布规则 回归测试" \
  --scope-id scp_demo \
  --limit 5 \
  --json
```

### 4.3 写入图片记忆

```bash
cargo run -p memory-cli -- remember-image \
  --scope-id scp_demo \
  --title "登录页截图" \
  --body "用于回忆 UI 状态" \
  --file /path/to/example.png \
  --json
```

### 4.4 导出 Agent Skill

```bash
cargo run -p memory-cli -- skills export \
  --target all \
  --output-dir ./dist/agent-skills \
  --force
```

### 4.5 检查 MCP 暴露情况

```bash
cargo run -p memory-cli -- mcp info
cargo run -p memory-cli -- mcp info --check-http
```

### 4.6 项目文档与短期上下文

```bash
memory-cli source create --name "Project docs" --source-kind local_docs --sync-mode index_only --local-root ./docs --json
memory-cli docs sync --source-id src_xxx --dry-run --json
memory-cli context upsert --session-id current --title "Current task" --body "Prepare docs cleanup." --json
```

## 5. HTTP / MCP 接入

默认服务地址：

```text
http://127.0.0.1:8080
```

常用 HTTP 接口：

- `GET /healthz`
- `GET /readyz`
- `GET /api/v1/meta`
- `POST /api/v1/memories`
- `POST /api/v1/images`
- `POST /api/v1/context/search`

MCP 常用入口：

- `GET /mcp/tools`
- `POST /mcp/tools/call`

## 6. 验证与测试

常用脚本：

```bash
./docs/scripts/test-required.sh
./docs/scripts/v1-acceptance.sh
./docs/scripts/v2_1-acceptance.sh
./docs/scripts/v2_4-acceptance.sh
./docs/scripts/write-test-reports.sh
./docs/scripts/security-report.sh
```

测试说明和报告目录见：

- [`tests/README.md`](../../tests/README.md)
- [`tests/reports/README.md`](../../tests/reports/README.md)

## 7. 常见排查路径

配置异常时：

- 先看 `config/app.toml`
- 再跑 `memory-cli config check`
- 必要时追加 `--database`

接口异常时：

- 看 `GET /healthz` 和 `GET /readyz`
- 看 `GET /api/v1/meta`
- 看 `GET /mcp/tools`

文档同步异常时：

- 先用 `docs sync --dry-run`
- 有 `missing` 或 `conflicts` 时先确认，不要静默覆盖

## 8. 相关文档

- [`README.md`](../../README.md)
- [`docs/runbook/install.md`](install.md)
- [`docs/api/README.md`](../api/README.md)
- [`docs/architecture/system-design.md`](../architecture/system-design.md)
- [`docs/agent-skills/README.md`](../agent-skills/README.md)
- [`config/README.md`](../../config/README.md)
