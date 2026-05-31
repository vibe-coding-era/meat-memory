# Meat Memory release-V1 部署包

本目录是 `release-V1` 的部署交付目录，只包含部署文档和安装程序，不包含源码。

English version: [README.en.md](./README.en.md)

## 交付内容

| 文件 | 用途 |
| --- | --- |
| `README.md` | 默认中文安装与运维说明 |
| `README.en.md` | English installation and operations guide |
| `DEPLOYMENT_PLAN.md` | release-V1 部署方案、发布流程和回滚策略 |
| `install.sh` | macOS / Linux 二进制安装脚本 |

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

## 可配置参数

| 环境变量 | 默认值 | 说明 |
| --- | --- | --- |
| `MEAT_MEMORY_REPO` | `vibe-coding-era/meat-memory` | GitHub 仓库 |
| `MEAT_MEMORY_VERSION` | `release-V1` | GitHub Release tag |
| `MEAT_MEMORY_RELEASE_BASE_URL` | GitHub Release 下载地址 | 私有镜像或内网制品库地址 |
| `MEAT_MEMORY_INSTALL_DIR` | `$HOME/.local/bin` | 二进制安装目录 |
| `MEAT_MEMORY_CONFIG_DIR` | `$HOME/.config/meat-memory` | 默认配置目录 |

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

## 发布前检查

发布 `release-V1` 前至少确认：

- GitHub Release tag 为 `release-V1`。
- 四个平台资产及 `.sha256` 文件已发布。
- 安装脚本在 macOS arm64、macOS x86_64、Linux amd64、Linux arm64 中至少覆盖目标发布平台。
- 生产环境凭据、数据库、OCR/ASR/video provider、benchmark evidence 等外部门禁已有正式证据或 release waiver。
- 本目录提交到 GitHub 时只包含部署文档和安装脚本，不包含源码。
