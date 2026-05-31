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

安装脚本默认从 GitHub Release 下载 `release-V1` 的预编译二进制包：

```bash
bash deploy/release-V1/install.sh
```

从远程仓库直接安装时：

```bash
curl -fsSL https://raw.githubusercontent.com/vibe-coding-era/meat-memory/release-V1/deploy/release-V1/install.sh | bash
```

默认安装到 `~/.local/bin`。如果该目录不在 `PATH` 中，请加入 shell 配置：

```bash
export PATH="$HOME/.local/bin:$PATH"
```

## 本地部署快速启动

如果你遇到下面两个错误，通常是因为执行了旧的源码开发命令，或者不在仓库根目录：

```text
cp: .env.example: No such file or directory
zsh: no such file or directory: ./docs/scripts/dev-up.sh
```

release-V1 部署包请使用本目录内的自包含入口：

```bash
cd deploy/release-V1
cp .env.example .env
./dev-up.sh
```

`dev-up.sh` 会读取同目录 `.env`，必要时先运行 `install.sh` 下载 release 二进制，然后在后台启动 `memory-app` 和 `memory-worker`。停止进程：

```bash
./dev-down.sh
```

默认 `.env` 指向 `127.0.0.1:5433` 的 PostgreSQL / pgvector；如果你的数据库在别处，请先修改 `.env` 中的 `MEAT_MEMORY_DATABASE_URL`。

注意：`./docs/scripts/dev-up.sh` 是源码仓库的 Docker 开发入口，不是 release-V1 二进制部署入口。

## 可配置参数

| 环境变量 | 默认值 | 说明 |
| --- | --- | --- |
| `MEAT_MEMORY_REPO` | `vibe-coding-era/meat-memory` | GitHub 仓库 |
| `MEAT_MEMORY_VERSION` | `release-V1` | GitHub Release tag |
| `MEAT_MEMORY_RELEASE_BASE_URL` | GitHub Release 下载地址 | 私有镜像或内网制品库地址 |
| `MEAT_MEMORY_INSTALL_DIR` | `$HOME/.local/bin` | 二进制安装目录 |
| `MEAT_MEMORY_CONFIG_DIR` | `$HOME/.config/meat-memory` | 默认配置目录 |
| `MEAT_MEMORY_ENV_FILE` | `deploy/release-V1/.env` | `dev-up.sh` 使用的环境变量文件 |

示例：

```bash
MEAT_MEMORY_INSTALL_DIR=/usr/local/bin \
MEAT_MEMORY_VERSION=release-V1 \
bash deploy/release-V1/install.sh
```

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
memory-cli --help
memory-app --help
memory-worker --help
```

如果使用默认配置目录，安装程序会在 `~/.config/meat-memory/app.toml` 写入发布包中的示例配置。已有配置不会被覆盖。

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
MEAT_MEMORY_VERSION=<previous-release-tag> bash deploy/release-V1/install.sh
```

如需完全移除：

```bash
rm -f ~/.local/bin/memory-cli ~/.local/bin/memory-app ~/.local/bin/memory-worker
```

配置和数据目录需要按实际部署策略单独备份或清理。

## 常见问题

### `cp: .env.example: No such file or directory`

先确认当前目录。release-V1 部署应进入部署目录后再复制：

```bash
cd deploy/release-V1
cp .env.example .env
```

### `./docs/scripts/dev-up.sh: no such file or directory`

不要在部署-only 包里运行源码开发脚本。改用：

```bash
cd deploy/release-V1
./dev-up.sh
```

## 发布前检查

发布 `release-V1` 前至少确认：

- GitHub Release tag 为 `release-V1`。
- 四个平台资产及 `.sha256` 文件已发布。
- 安装脚本在 macOS arm64、macOS x86_64、Linux amd64、Linux arm64 中至少覆盖目标发布平台。
- 生产环境凭据、数据库、OCR/ASR/video provider、benchmark evidence 等外部门禁已有正式证据或 release waiver。
- 本目录提交到 GitHub 时只包含部署文档和安装脚本，不包含源码。
