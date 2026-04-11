# Meat Memory 配置目录

本目录现在只保留应用运行配置，避免维护多份重复 TOML。

## 文件说明

- `app.toml`: 唯一的应用主配置。本地、Docker、云端都从它启动。
- `app.local.toml`: 建议的本地覆盖文件名，由 `memory-cli tui init --interactive --write-config config/app.local.toml` 生成，不建议提交真实私有配置。
- `app.generated.toml`: 建议的临时生成文件名，适合验证 TUI 输出。

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
