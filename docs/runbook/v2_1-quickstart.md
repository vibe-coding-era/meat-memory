# V2.1 Quickstart Runbook

本 runbook 用于安装完成后，快速把 `CLI / MCP / Agent skill / TUI setup wizard` 跑通。

## 1. 先检查配置

```bash
cargo run -p memory-cli -- config check
cargo run -p memory-cli -- config check --database
```

建议：

- 本地只想先验证模型路由时，可先跑 `config check`
- PostgreSQL 已启动时，再补跑 `config check --database`

## 2. 查看 MCP 接入信息

```bash
cargo run -p memory-cli -- mcp info
cargo run -p memory-cli -- mcp info --check-http
cargo run -p memory-cli -- mcp info --json
```

说明：

- `mcp info` 用于查看当前配置下的 MCP 地址和工具清单
- `mcp info --check-http` 会额外检查当前 `/mcp/tools` 是否可达
- 如果服务未启动，`HTTP check` 会返回 `unreachable: ...`

## 3. 运行安装向导

预览模式：

```bash
cargo run -p memory-cli -- tui init
```

交互向导：

```bash
cargo run -p memory-cli -- tui init --interactive
```

交互向导当前流程：

1. 先选语言：`中文 / English`
2. 再选 setup profile：`Local default / MCP-ready / Markdown-first`
3. 配置 MCP、数据库、Markdown、Assets
4. 用编号菜单选择 reasoning / extraction / vision / embedding；候选项会显示 alias、provider、deployment、locale、priority
5. 选择是否做数据库检查、是否写配置、最后确认

如果向导写出了配置文件，最后会提示类似命令：

```bash
MEAT_MEMORY_CONFIG=config/app.generated.toml memory-cli config check
```

## 4. 导出 Agent skills

```bash
cargo run -p memory-cli -- skills export --target all --output-dir ./dist/agent-skills --force
```

导出包包含：

- `SKILL.md`
- `agents/openai.yaml`
- `assets/icon.svg`

## 5. 运行 V2.1 验收

```bash
./docs/scripts/v2_1-acceptance.sh
```

当前覆盖：

- `config check`
- `mcp info`
- `tui init`
- `tui init --interactive` 中文流程
- `skills export`

最新报告位置：

- [`tests/reports/e2e/latest/v2_1-acceptance.txt`](/Users/Rou/dev_projects/meat-memory/tests/reports/e2e/latest/v2_1-acceptance.txt)
- [`tests/reports/latest-run.md`](/Users/Rou/dev_projects/meat-memory/tests/reports/latest-run.md)
