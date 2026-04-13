# Meat Memory 配置目录

本目录只保留应用运行配置，目标是减少重复 TOML，统一本地、Docker 和云端的启动入口。

## 文件说明

- `app.toml`：唯一主配置，本地、Docker、云端都基于它启动
- `app.local.toml`：建议的本地覆盖文件名，适合个人开发环境
- `app.generated.toml`：建议的临时生成文件名，适合验证 TUI 输出

## 覆盖方式

优先使用环境变量覆盖部署差异，而不是复制多份配置文件：

```bash
MEAT_MEMORY_CONFIG=config/app.toml
MEAT_MEMORY_SERVER_BIND=127.0.0.1:8080
MEAT_MEMORY_DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5433/meat_memory_dev
MEAT_MEMORY_MARKDOWN_ROOT=./docs
MEAT_MEMORY_ASSETS_ROOT=./storage/assets
MEAT_MEMORY_ENABLE_MCP=false
```

Docker Compose 也使用同一个 `app.toml`，通过 `MEAT_MEMORY_*` 环境变量覆盖容器内地址与目录。

## TUI 使用方式

```bash
cargo run -p memory-cli -- tui init --interactive --write-config config/app.local.toml
MEAT_MEMORY_CONFIG=config/app.local.toml cargo run -p memory-cli -- config check
```

## 推荐约定

- 仓库内提交公共默认配置：`config/app.toml`
- 本地私有差异落到：`config/app.local.toml`
- CI、容器和云端差异通过环境变量注入
- 不要再复制出 `docker.toml`、`cloud.toml` 这类平行配置文件

## 为什么有些配置不在本目录

以下文件有工具链固定约定，不能强行移动到 `config/`，否则工具可能读取不到：

- `.cargo/config.toml`
- `Cargo.toml`
- `rust-toolchain.toml`
- `rustfmt.toml`
- `clippy.toml`
- `.github/workflows/*.yml`
- `compose.yaml`

它们保留在工具期望的位置；应用运行配置统一放在本目录。
