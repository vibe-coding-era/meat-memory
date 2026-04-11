# V1 CLI 使用说明

二进制包名：

```text
memory-cli
```

## 1. 常用命令

| 命令 | 用途 |
|---|---|
| `doctor` | 输出环境检查提示 |
| `print-plan` | 输出任务清单位置 |
| `config show` | 查看当前配置摘要 |
| `config check` | 检查当前配置与模型路由是否可用 |
| `mcp info` | 查看 MCP 是否启用、工具地址和工具清单 |
| `skills export` | 导出 Agent skill 模板到指定目录 |
| `tui init` | 打开安装后初始化配置面板预览 |
| `serve` | 启 HTTP 服务，可按配置启 MCP |
| `remember` | 写入文本记忆 |
| `remember-image` | 写入图片记忆 |
| `search` | 搜索上下文 |

## 2. 配置查看与检查

安装或复制配置后，建议先检查当前配置：

```bash
cargo run -p memory-cli -- config show
cargo run -p memory-cli -- config check
cargo run -p memory-cli -- config check --database
```

如果需要给脚本或 Agent 读取，可加 `--json`：

```bash
cargo run -p memory-cli -- config show --json
cargo run -p memory-cli -- config check --json
```

查看 MCP 接入信息：

```bash
cargo run -p memory-cli -- mcp info
cargo run -p memory-cli -- mcp info --json
```

导出 Agent skill：

```bash
cargo run -p memory-cli -- skills export --target all --output-dir ./dist/agent-skills --force
cargo run -p memory-cli -- skills export --target codex --output-dir /tmp/codex-skills --force
```

导出的完整包默认包含：

- `SKILL.md`
- `agents/openai.yaml`
- `assets/icon.svg`

其中 `agents/openai.yaml` 当前会带出这些 UI metadata：

- `display_name`
- `short_description`
- `icon_small`
- `icon_large`
- `brand_color`
- `default_prompt`

可选 target：

- `codex`
- `claude-code`
- `execution-agent`
- `all`

这些命令会读取 `MEAT_MEMORY_CONFIG` 指定的配置；未设置时默认读取 `config/default.toml`。

安装后也可以先打开 TUI 初始化面板预览：

```bash
cargo run -p memory-cli -- tui init
cargo run -p memory-cli -- tui init --check-database
cargo run -p memory-cli -- tui init --json
```

当前 `tui init` 会展示 LLM 路由、存储、MCP 状态和推荐下一步，也可以生成推荐配置文件：

```bash
cargo run -p memory-cli -- tui init \
  --enable-mcp \
  --reasoning-primary qwen_reasoning \
  --markdown-root ./docs/default \
  --assets-root ./storage/assets \
  --write-config config/local.generated.toml
```

常用覆盖项：

- `--enable-mcp` / `--disable-mcp`
- `--database-url <url>`
- `--markdown-root <path>`
- `--assets-root <path>`
- `--reasoning-primary <alias>`
- `--extraction-primary <alias>`
- `--vision-primary <alias>`
- `--embedding-primary <alias>`
- `--write-config <path>`
- `--force`
- `--check-database`

写入后可这样验证：

```bash
MEAT_MEMORY_CONFIG=config/local.generated.toml cargo run -p memory-cli -- config check
MEAT_MEMORY_CONFIG=config/local.generated.toml cargo run -p memory-cli -- config check --database
```

## 3. 文本写入

```bash
cargo run -p memory-cli -- remember \
  --scope-id scp_cli_demo \
  --title "发布约束" \
  --body "发布前必须通过回归与验收。" \
  --memory-kind constraint \
  --json
```

## 4. 图片写入

```bash
cargo run -p memory-cli -- remember-image \
  --scope-id scp_cli_demo \
  --title "登录页截图" \
  --body "Codex 登录页截图" \
  --file ./samples/login.png \
  --json
```

返回中会带：

- `asset_uri`
- `vision_caption`
- `vision_model_alias`
- `llm_notice`

如果主视觉 LLM 不可用且自动切到了 fallback，CLI 会：

- 在 JSON 输出里返回 `llm_notice`
- 在文本输出里追加 `LLM Notice: ...`

提示文案格式：

```text
{某}LLM 不可用，已经切换到{新}LLM
```

## 5. 搜索

```bash
cargo run -p memory-cli -- search "发布约束 回归" \
  --scope-id scp_cli_demo \
  --limit 5 \
  --json
```

## 6. 启服务

```bash
cargo run -p memory-cli -- serve --bind 127.0.0.1:8080
```

说明：

- 如果配置开启 `enable_mcp = true`，则会同时挂出 `/mcp/tools` 和 `/mcp/tools/call`。
- 如果关闭 `enable_mcp`，则只暴露 HTTP API。

## 7. 配置文件

默认读取：

```text
config/default.toml
```

你也可以覆盖：

```bash
MEAT_MEMORY_CONFIG=config/local.toml cargo run -p memory-cli -- search "上线规则" --json
```
