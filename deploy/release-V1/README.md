# Meat Memory release-V1 部署包

本目录是 `release-V1` 的部署交付目录，只包含部署文档和安装程序，不包含源码。

English version: [README.en.md](./README.en.md)

## 交付内容

| 文件 | 用途 |
| --- | --- |
| `README.md` | 默认中文安装与运维说明 |
| `README.en.md` | English installation and operations guide |
| `DEPLOYMENT_PLAN.md` | release-V1 部署方案、发布流程和回滚策略 |
| `.env.example` | 二进制部署环境变量模板 |
| `install.sh` | macOS / Linux 二进制安装脚本 |
| `dev-up.sh` | 使用 release 二进制启动本地 app / worker |
| `dev-down.sh` | 停止 `dev-up.sh` 启动的本地进程 |

## 安装方式

新用户和服务器部署必须从服务端下载安装程序，不要求本机已有源码仓库或 `deploy/release-V1` 目录。推荐一行命令：

```bash
curl -fsSL https://raw.githubusercontent.com/vibe-coding-era/meat-memory/release-V1/deploy/release-V1/install.sh | bash -s -- --skip-tui --verify-write
```

这条命令会下载 `release-V1` 对应平台二进制，安装 `memory-cli`、`memory-app`、`memory-worker`，生成 markdown-only 快速开始配置，并验证第一条 memory 可以写入和检索。

GitHub Release 页面会自动显示 `Source code (zip)` 和 `Source code (tar.gz)`，那是 GitHub 生成的源码快照，不是安装包。普通用户不要下载这两个源码包；请选择 `meat-memory-<os>-<arch>.tar.gz` 二进制资产，或直接使用上面的一行安装命令。

如果你希望先保存脚本再审阅，可以用等价的展开流程：

```bash
mkdir -p ~/meat-memory-release-V1
cd ~/meat-memory-release-V1
curl -fsSLO https://raw.githubusercontent.com/vibe-coding-era/meat-memory/release-V1/deploy/release-V1/install.sh
chmod +x install.sh
./install.sh --check
./install.sh --skip-tui --verify-write
```

如果环境检测提示缺少 `curl`、`tar`、`awk`、`coreutils` 等基础工具，可以先看脚本输出的系统包管理器命令；确认后再让安装器尝试补齐：

```bash
./install.sh --install-deps
```

不要把远程脚本直接 pipe 到 `bash --install-deps`；需要系统依赖时，先保存脚本、审阅脚本和输出的包管理器命令，再显式执行 `./install.sh --install-deps`。

安装脚本默认从 GitHub Release 下载 `release-V1` 的预编译二进制包。首次安装会生成用户目录下的 markdown-only 快速开始配置，默认不要求本机已有 PostgreSQL；配置和数据目录分别位于 `~/.config/meat-memory` 与 `~/.local/share/meat-memory`。

如果希望安装后立即验证第一条 memory 写入，可以加 `--verify-write`：

```bash
./install.sh --skip-tui --verify-write
```

安装器会使用安装后的 `memory-cli remember` 写入一条 quickstart memory，再用 `memory-cli search` 验证可检索。安装后在有交互终端时默认启动 `memory-cli tui init --interactive`；如果是在 CI、自动化脚本或只想安装二进制，可以跳过 TUI：

```bash
./install.sh --skip-tui
```

默认安装到 `~/.local/bin`。如果该目录不在 `PATH` 中，请加入 shell 配置：

```bash
export PATH="$HOME/.local/bin:$PATH"
```

## 服务端部署快速启动

如果你遇到下面两个错误，通常是因为执行了旧的源码开发命令，或者把文档里的仓库内路径当成了新用户安装路径：

```text
cp: .env.example: No such file or directory
zsh: no such file or directory: ./docs/scripts/dev-up.sh
```

服务器上请直接从 GitHub raw 下载部署辅助脚本并完成安装验证，不要假设本机有源码目录：

```bash
mkdir -p ~/meat-memory-release-V1 && cd ~/meat-memory-release-V1 && BASE_URL=https://raw.githubusercontent.com/vibe-coding-era/meat-memory/release-V1/deploy/release-V1 && for f in install.sh .env.example dev-up.sh dev-down.sh; do curl -fsSLO "$BASE_URL/$f"; done && chmod +x install.sh dev-up.sh dev-down.sh && ./install.sh --skip-tui --verify-write
```

上面这条命令只负责下载安装、环境检查和写入第一条 markdown-only memory，不会启动依赖数据库的长驻服务。

如果要启动 `memory-app` 和 `memory-worker`，先创建 `.env`：

```bash
cp .env.example .env
./dev-up.sh
```

默认 `.env.example` 使用 markdown-only quickstart：`MEAT_MEMORY_ENABLE_PG=0`，不要求本机已有 PostgreSQL。停止进程：

```bash
./dev-down.sh
```

如果你要启用 PostgreSQL / pgvector，请先启动数据库或把 `.env` 中的 `MEAT_MEMORY_DATABASE_URL` 改为可访问地址，再设置 `MEAT_MEMORY_ENABLE_PG=1`。`dev-up.sh` 只会在 PostgreSQL 启用时检查数据库 TCP 端口；如果你只想验证脚本路径，可设置 `MEAT_MEMORY_SKIP_DB_CHECK=1` 跳过该检查。
命令行传入的 `MEAT_MEMORY_*` 环境变量优先级高于 `.env`，例如：

```bash
MEAT_MEMORY_ENABLE_PG=0 ./dev-up.sh
```

注意：`./docs/scripts/dev-up.sh` 是源码仓库的 Docker 开发入口，不是 release-V1 二进制部署入口。

## 可配置参数

| 环境变量 | 默认值 | 说明 |
| --- | --- | --- |
| `MEAT_MEMORY_REPO` | `vibe-coding-era/meat-memory` | GitHub 仓库 |
| `MEAT_MEMORY_VERSION` | `release-V1` | GitHub Release tag |
| `MEAT_MEMORY_RELEASE_BASE_URL` | GitHub Release 下载地址 | 私有镜像或内网制品库地址 |
| `MEAT_MEMORY_INSTALL_DIR` | `$HOME/.local/bin` | 二进制安装目录 |
| `MEAT_MEMORY_CONFIG_DIR` | `$HOME/.config/meat-memory` | 默认配置目录 |
| `MEAT_MEMORY_DATA_DIR` | `$HOME/.local/share/meat-memory` | quickstart 数据目录 |
| `MEAT_MEMORY_MARKDOWN_ROOT` | `$MEAT_MEMORY_DATA_DIR/markdown` | quickstart markdown memory 目录 |
| `MEAT_MEMORY_ASSETS_ROOT` | `$MEAT_MEMORY_DATA_DIR/assets` | quickstart 资产目录 |
| `MEAT_MEMORY_KEY_STORE_PATH` | `$MEAT_MEMORY_DATA_DIR/keys/default-key.toml` | quickstart key 配置路径 |
| `MEAT_MEMORY_SYNC_STATE_PATH` | `$MEAT_MEMORY_DATA_DIR/sync/state.json` | quickstart sync 状态路径 |
| `MEAT_MEMORY_DEFAULT_ENABLE_PG` | `0` | 首次生成配置时是否默认启用 PostgreSQL |
| `MEAT_MEMORY_OVERWRITE_CONFIG` | `0` | `1` 时覆盖已有 `app.toml` |
| `MEAT_MEMORY_TUI_CONFIG` | `$MEAT_MEMORY_CONFIG_DIR/app.local.toml` | TUI 推荐写出的本地配置 |
| `MEAT_MEMORY_RUN_TUI` | `auto` | `auto` 有交互终端时运行 TUI；`1` 强制；`0` 跳过 |
| `MEAT_MEMORY_INSTALL_MISSING_DEPS` | `0` | `1` 时等同 `--install-deps` |
| `MEAT_MEMORY_VERIFY_SCOPE_ID` | `scp_release_v1_quickstart` | `--verify-write` 使用的 scope |
| `MEAT_MEMORY_ENV_FILE` | `<部署脚本目录>/.env` | `dev-up.sh` 使用的环境变量文件 |

示例：

```bash
MEAT_MEMORY_INSTALL_DIR=/usr/local/bin \
MEAT_MEMORY_VERSION=release-V1 \
./install.sh --skip-tui --verify-write
```

说明：`--check` 不会下载或安装 release 二进制，但会创建安装目录和配置目录，用来确认路径可写。

## 支持平台

| 系统 | 架构 | Release asset |
| --- | --- | --- |
| macOS | arm64 | `meat-memory-darwin-arm64.tar.gz` |
| macOS | x86_64 | `meat-memory-darwin-amd64.tar.gz` |
| Linux | arm64 / aarch64 | `meat-memory-linux-arm64.tar.gz` |
| Linux | x86_64 / amd64 | `meat-memory-linux-amd64.tar.gz` |

每个二进制包必须同时发布对应的 `.sha256` 文件。安装程序会先校验 checksum，再复制可执行文件。

## 安装后验证

```bash
export MEAT_MEMORY_CONFIG="$HOME/.config/meat-memory/app.toml"
memory-cli --help
memory-cli config check
command -v memory-app
command -v memory-worker
```

写入并检索第一条 memory：

```bash
export MEAT_MEMORY_CONFIG="$HOME/.config/meat-memory/app.toml"
memory-cli remember \
  --scope-id scp_release_v1_quickstart \
  --title "Release V1 install smoke" \
  --body "Meat Memory release-V1 installer wrote this quickstart memory." \
  --memory-kind procedure \
  --json

memory-cli search "quickstart memory" --scope-id scp_release_v1_quickstart --limit 5 --json
```

`memory-app` 和 `memory-worker` 是长驻服务入口，验证安装时先检查二进制存在；正式启动请使用 `dev-up.sh`、systemd、launchd 或你的进程管理器。

如果使用默认配置目录，安装程序会在 `~/.config/meat-memory/app.toml` 写入发布包中的示例配置，并改写为用户目录下的 markdown-only 快速开始配置。已有配置不会被覆盖，除非设置 `MEAT_MEMORY_OVERWRITE_CONFIG=1`。`memory-cli config check` 如果报告 warning，安装器会继续执行并引导你进入 TUI 配置；这类 warning 通常表示本地数据库、模型或路径还未配置完成，不代表二进制安装失败。TUI 配置默认建议写入 `~/.config/meat-memory/app.local.toml`，写出后用：

```bash
MEAT_MEMORY_CONFIG="$HOME/.config/meat-memory/app.local.toml" memory-cli config check
```

## 服务部署建议

release-V1 推荐三种部署形态：

| 场景 | 推荐方式 |
| --- | --- |
| 本机试用、单用户 CLI | 直接运行 `memory-cli` |
| 长驻 HTTP / MCP 服务 | 使用 `memory-app`，配合 systemd、launchd 或进程管理器 |
| 后台任务 | 使用 `memory-worker`，独立进程管理和日志采集 |

生产或团队环境建议：

- 使用独立服务账号运行 `memory-app` 和 `memory-worker`。
- 配置文件和数据目录不要放在安装目录中。
- 密钥和数据库连接串通过环境变量、secret manager 或系统级凭据管理注入。
- 发布前确认 `release-V1` GitHub Release 资产和 checksum 均来自受信 CI。

## 升级与回滚

升级同样运行安装脚本，并指定新的 `MEAT_MEMORY_VERSION`。安装脚本会覆盖二进制文件，不覆盖已有配置。

回滚时指定上一个已验证的 release tag：

```bash
MEAT_MEMORY_VERSION=<previous-release-tag> ./install.sh --skip-tui
```

如需完全移除：

```bash
rm -f ~/.local/bin/memory-cli ~/.local/bin/memory-app ~/.local/bin/memory-worker
```

配置和数据目录需要按实际部署策略单独备份或清理。

## 常见问题

### `cp: .env.example: No such file or directory`

先下载部署辅助文件并确认当前目录：

```bash
mkdir -p ~/meat-memory-release-V1
cd ~/meat-memory-release-V1
BASE_URL=https://raw.githubusercontent.com/vibe-coding-era/meat-memory/release-V1/deploy/release-V1
curl -fsSLO "$BASE_URL/.env.example"
cp .env.example .env
```

### `./docs/scripts/dev-up.sh: no such file or directory`

不要在部署-only 包里运行源码开发脚本。下载并使用服务端部署入口：

```bash
mkdir -p ~/meat-memory-release-V1
cd ~/meat-memory-release-V1
BASE_URL=https://raw.githubusercontent.com/vibe-coding-era/meat-memory/release-V1/deploy/release-V1
curl -fsSLO "$BASE_URL/install.sh"
curl -fsSLO "$BASE_URL/.env.example"
curl -fsSLO "$BASE_URL/dev-up.sh"
curl -fsSLO "$BASE_URL/dev-down.sh"
chmod +x install.sh dev-up.sh dev-down.sh
./dev-up.sh
```

## 发布前检查

发布 `release-V1` 前至少确认：

- GitHub Release tag 为 `release-V1`。
- 四个平台资产及 `.sha256` 文件已发布。
- `install.sh --check` 在目标平台通过。
- 安装脚本在 macOS arm64、macOS x86_64、Linux amd64、Linux arm64 中至少覆盖目标发布平台。
- 安装后已完成 `memory-cli tui init --interactive` 或明确使用 `--skip-tui` 跳过。
- 生产环境凭据、数据库、OCR/ASR/video provider、benchmark evidence 等外部门禁已有正式证据或 release waiver。
- 本目录提交到 GitHub 时只包含部署文档和安装脚本，不包含源码。
