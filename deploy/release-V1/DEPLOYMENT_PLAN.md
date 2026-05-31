# Meat Memory release-V1 部署方案

## 目标

`release-V1` 的部署交付目标是提供一个不包含源码的安装入口，让用户可以从 GitHub Release 获取受信的二进制包并完成本地或服务器部署。

本方案默认中文说明；英文安装说明见 [README.en.md](./README.en.md)。

## 交付边界

包含：

- macOS / Linux 二进制安装脚本。
- 二进制部署专用 `.env.example`、`dev-up.sh` 和 `dev-down.sh`。
- 中文和英文 README。
- 发布、安装、验证、升级、回滚说明。

不包含：

- Rust、前端、SDK、测试或迁移源码。
- 真实生产密钥、数据库连接串或第三方 provider token。
- 未经 CI 产出的本地构建二进制。

## Release 资产约定

GitHub 仓库：

```text
vibe-coding-era/meat-memory
```

默认 release tag：

```text
release-V1
```

必须发布的资产：

| 平台 | 资产 | checksum |
| --- | --- | --- |
| Linux amd64 | `meat-memory-linux-amd64.tar.gz` | `meat-memory-linux-amd64.tar.gz.sha256` |
| Linux arm64 | `meat-memory-linux-arm64.tar.gz` | `meat-memory-linux-arm64.tar.gz.sha256` |
| macOS amd64 | `meat-memory-darwin-amd64.tar.gz` | `meat-memory-darwin-amd64.tar.gz.sha256` |
| macOS arm64 | `meat-memory-darwin-arm64.tar.gz` | `meat-memory-darwin-arm64.tar.gz.sha256` |

每个 tarball 内应包含：

```text
${ASSET_NAME}/bin/memory-cli
${ASSET_NAME}/bin/memory-app
${ASSET_NAME}/bin/memory-worker
${ASSET_NAME}/config/app.toml
${ASSET_NAME}/README.txt
```

## 安装流程

1. 安装程序先执行环境检测：OS / CPU 架构、下载工具、tar、awk、install、checksum 工具、安装目录和配置目录可写性。
2. 如果缺少基础工具，默认只输出修复命令；用户显式传入 `--install-deps` 或 `MEAT_MEMORY_INSTALL_MISSING_DEPS=1` 后，才尝试通过系统包管理器补齐。
3. 根据平台选择对应的 release tarball。
4. 下载 tarball 和 `.sha256` 文件。
5. 计算本地 sha256，并与发布 checksum 对比。
6. 解压发布包。
7. 将 `memory-cli`、`memory-app`、`memory-worker` 安装到 `MEAT_MEMORY_INSTALL_DIR`。
8. 如果 `MEAT_MEMORY_CONFIG_DIR/app.toml` 不存在，复制发布包中的示例配置，并改写为用户目录下的 markdown-only 快速开始配置：`MEAT_MEMORY_DATA_DIR/markdown`、`MEAT_MEMORY_DATA_DIR/assets`、`MEAT_MEMORY_DATA_DIR/keys`、`MEAT_MEMORY_DATA_DIR/sync`、`enable_pg = false`。
9. 运行 `memory-cli --help` 和 `memory-cli config check` 做基础验证；`config check` warning 不阻断二进制安装，后续由 TUI 配置处理。
10. 如果传入 `--verify-write`，使用安装后的 `memory-cli remember` 写入一条 quickstart memory，并用 `memory-cli search` 校验可检索。
11. 如果存在交互终端，默认进入 `memory-cli tui init --interactive --write-config <app.local.toml>` 完成本地配置；CI 或自动化场景可使用 `--skip-tui`。

安装程序不克隆仓库、不运行源码构建、不写入源码目录。

推荐命令：

```bash
bash deploy/release-V1/install.sh --check
bash deploy/release-V1/install.sh --skip-tui --verify-write
```

`--check` 不下载、不安装 release 资产，但会创建安装目录和配置目录以确认可写性。
如环境检测提示缺少基础工具，审阅安装器输出后再运行 `bash deploy/release-V1/install.sh --install-deps`。

## 本地启动流程

release-V1 二进制部署不使用源码仓库的 `./docs/scripts/dev-up.sh`。本地验证使用部署目录内的入口：

```bash
cd deploy/release-V1
cp .env.example .env
./dev-up.sh
```

`dev-up.sh` 的行为：

1. 如果 `.env` 不存在，自动从 `.env.example` 复制一份。
2. 读取 `.env` 并将相对路径解析到 `deploy/release-V1/`。
3. 如果 `bin/memory-cli`、`bin/memory-app` 或 `bin/memory-worker` 不存在，调用 `install.sh` 安装 release 二进制。
4. 创建 `data/`、`logs/` 和 `run/` 目录。
5. 检查 `MEAT_MEMORY_DATABASE_URL` 指向的 PostgreSQL / pgvector TCP 端口；如需脚本-only smoke，可设置 `MEAT_MEMORY_SKIP_DB_CHECK=1` 跳过。
6. 后台启动 `memory-app` 和 `memory-worker`，pid 写入 `run/`，日志写入 `logs/`。

默认 `.env` 使用 `127.0.0.1:5433` 的 PostgreSQL / pgvector。生产或非本机数据库部署前，应先修改 `MEAT_MEMORY_DATABASE_URL`。
命令行传入的 `MEAT_MEMORY_*` 环境变量优先级高于 `.env`，便于临时覆盖数据库或跳过 DB preflight。

停止：

```bash
cd deploy/release-V1
./dev-down.sh
```

## 推荐目录

单用户默认：

```text
~/.local/bin
~/.config/meat-memory
~/.local/share/meat-memory
```

服务器建议：

```text
/opt/meat-memory/bin
/etc/meat-memory
/var/lib/meat-memory
/var/log/meat-memory
```

生产部署时建议使用独立服务账号，并将运行数据、日志、配置和二进制目录分离。

## 服务运行方式

### CLI

适合个人本机或自动化脚本：

```bash
export MEAT_MEMORY_CONFIG="$HOME/.config/meat-memory/app.toml"
memory-cli --help
memory-cli config check
```

### HTTP / MCP 服务

适合长驻服务：

```bash
memory-app
```

建议由 systemd、launchd、supervisor 或容器平台托管。服务环境变量由部署系统注入，不写入安装脚本。

### Worker

适合后台任务：

```bash
memory-worker
```

建议和 `memory-app` 分开管理，便于独立扩缩容、重启和采集日志。

## 发布步骤

1. 在发布分支上确认 release gate 已通过，或已获得正式 release waiver。
2. 创建 `release-V1` tag 并触发 GitHub Actions release workflow。
3. 确认 GitHub Release 中存在四个平台 tarball 和 checksum。
4. 在目标平台运行 `install.sh`。
5. 执行安装后验证：

```bash
export MEAT_MEMORY_CONFIG="$HOME/.config/meat-memory/app.toml"
memory-cli --help
memory-cli config check
command -v memory-app
command -v memory-worker
memory-cli remember --scope-id scp_release_v1_quickstart --title "Release V1 install smoke" --body "Meat Memory release-V1 installer wrote this quickstart memory." --memory-kind procedure --json
memory-cli search "quickstart memory" --scope-id scp_release_v1_quickstart --limit 5 --json
```

6. 按部署环境启动服务并检查日志。
7. 记录部署版本、checksum、部署时间和操作者。

## 回滚方案

回滚优先使用上一版已验证 release tag：

```bash
MEAT_MEMORY_VERSION=<previous-release-tag> bash install.sh
```

回滚检查：

- 重新执行 `memory-cli --help`。
- 检查服务进程已使用旧版本二进制重启。
- 检查日志中无启动失败、配置解析失败或数据库连接失败。
- 如果数据结构发生变化，按对应版本的数据迁移或备份策略处理。

## 安全与合规

- 安装脚本只处理二进制包和示例配置，不接收或保存真实密钥。
- 生产密钥必须由外部凭据系统注入。
- 下载的 tarball 必须通过 sha256 校验。
- 内网部署可通过 `MEAT_MEMORY_RELEASE_BASE_URL` 指向企业制品库。
- 对外发布前必须确认外部依赖证据：SaaS connector credentials、public benchmark dataset/hash、OCR/ASR/video provider refs、release DB、Web Manager URL/auth/browser proof、generic security、compatibility evidence，或正式 release waiver。

## 验收标准

部署目录验收：

- `README.md` 为默认中文说明。
- `README.en.md` 提供英文说明。
- `.env.example`、`dev-up.sh`、`dev-down.sh` 提供部署目录内的自包含启动路径。
- `install.sh` 通过 shell 语法检查。
- `install.sh --check` 能在不下载 release 资产的情况下完成环境检测。
- `install.sh --skip-tui --verify-write` 能用 release 二进制写入并检索一条 markdown-only memory。
- 安装后能进入 TUI 配置，或通过 `--skip-tui` 明确跳过。
- 安装脚本不包含 `git clone`、`cargo build` 或源码路径安装逻辑。
- 提交到 GitHub 时只包含 `deploy/release-V1/` 下的部署类文件。

发布验收：

- GitHub Release 资产完整。
- checksum 校验通过。
- 目标平台安装成功。
- 基础命令和 `remember/search` 写入验证成功。
- 外部发布门禁已通过或存在正式 waiver。
