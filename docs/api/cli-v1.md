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
| `serve` | 启 HTTP 服务，可按配置启 MCP |
| `remember` | 写入文本记忆 |
| `remember-image` | 写入图片记忆 |
| `search` | 搜索上下文 |

## 2. 文本写入

```bash
cargo run -p memory-cli -- remember \
  --scope-id scp_cli_demo \
  --title "发布约束" \
  --body "发布前必须通过回归与验收。" \
  --memory-kind constraint \
  --json
```

## 3. 图片写入

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

## 4. 搜索

```bash
cargo run -p memory-cli -- search "发布约束 回归" \
  --scope-id scp_cli_demo \
  --limit 5 \
  --json
```

## 5. 启服务

```bash
cargo run -p memory-cli -- serve --bind 127.0.0.1:8080
```

说明：

- 如果配置开启 `enable_mcp = true`，则会同时挂出 `/mcp/tools` 和 `/mcp/tools/call`。
- 如果关闭 `enable_mcp`，则只暴露 HTTP API。

## 6. 配置文件

默认读取：

```text
config/default.toml
```

你也可以覆盖：

```bash
MEAT_MEMORY_CONFIG=config/local.toml cargo run -p memory-cli -- search "上线规则" --json
```
