# 安装与部署入口

本文档面向准备安装或部署 `Meat Memory` 的用户，给出当前可用入口和推荐顺序。普通用户只需要“新用户一行安装”或“GitHub Release 二进制包”部分；源码构建、打包和研发脚本只适合发布维护者。

## 1. 推荐路径

### 新用户一行安装

不需要克隆源码，也不需要运行 `./docs/scripts/*`。直接从服务端下载安装器和 release 二进制：

```bash
curl -fsSL https://raw.githubusercontent.com/vibe-coding-era/meat-memory/release-V1/deploy/release-V1/install.sh | bash -s -- --skip-tui --verify-write
```

这条命令会安装 `memory-cli`、`memory-app`、`memory-worker`，生成 markdown-only 快速开始配置，并写入和检索一条 quickstart memory。

注意：GitHub Release 页面自动显示的 `Source code (zip)` 和 `Source code (tar.gz)` 是源码快照，不是新用户安装包。普通用户应使用一行安装命令，或下载 `meat-memory-<os>-<arch>.tar.gz` 二进制资产。

如果命令完成后 shell 找不到 `memory-cli`，先把默认安装目录加入 `PATH`：

```bash
export PATH="$HOME/.local/bin:$PATH"
```

### 开发者源码试跑

已经克隆源码并需要本地开发时，再看 [`usage-guide.md`](usage-guide.md)。新用户安装不要运行源码开发脚本。

如果你要给 Agent 使用，安装完成后运行：

```bash
memory-cli skills export --target all --output-dir ./dist/agent-skills --force
```

下面的 skill bundle 构建命令仅适合已经克隆源码仓库的发布维护者：

```bash
./docs/scripts/build-agent-skills-bundle.sh all ./dist/release
```

## 2. 安装方式

### cargo install

仅适合已经克隆源码并安装 Rust toolchain 的开发者：

```bash
cargo install --path crates/memory-cli --locked
memory-cli --help
```

安装后建议立刻执行：

```bash
memory-cli config check
memory-cli mcp info
memory-cli skills export --target all --output-dir ./dist/agent-skills --force
```

### GitHub Release 二进制包

普通用户优先使用一行安装命令：

```bash
curl -fsSL https://raw.githubusercontent.com/vibe-coding-era/meat-memory/release-V1/deploy/release-V1/install.sh | bash -s -- --skip-tui --verify-write
```

当前 release 流水线会产出：

- `meat-memory-linux-amd64.tar.gz`
- `meat-memory-linux-arm64.tar.gz`
- `meat-memory-darwin-amd64.tar.gz`
- `meat-memory-darwin-arm64.tar.gz`
- 对应的 `.sha256`
- `agent-skills-bundle.zip`

包内核心二进制：

- `memory-cli`
- `memory-app`
- `memory-worker`

### npm 与 Homebrew

这两条仍是后续接入点，原因是正式仓库地址、release 下载域名和包名还需要最终确定。

当前不要在文档里把它们当成已发布渠道使用。

Npm 包装层骨架已放在：

- [`../../packaging/npm/README.md`](../../packaging/npm/README.md)
- [`../../packaging/npm/package.json`](../../packaging/npm/package.json)

Homebrew 方案和 Formula 模板已放在：

- [`../../packaging/homebrew/README.md`](../../packaging/homebrew/README.md)
- [`../../packaging/homebrew/meat-memory.rb.template`](../../packaging/homebrew/meat-memory.rb.template)

## 3. 部署方式

### Docker Compose

仅适合源码开发环境，入口见 [`usage-guide.md`](usage-guide.md)。

默认镜像名：

- `meat-memory-app:local`
- `meat-memory-worker:local`

### 单机 systemd

适合一台 Linux 主机自托管。

入口：

- [`systemd-deploy.md`](systemd-deploy.md)
- [`../../infra/systemd/README.md`](../../infra/systemd/README.md)

核心文件：

- `infra/systemd/meat-memory-app.service`
- `infra/systemd/meat-memory-worker.service`
- `infra/systemd/meat-memory.env.example`

### Kubernetes / Helm

入口：

- [`cloud-deploy-v1.md`](cloud-deploy-v1.md)
- [`../../infra/helm/meat-memory/values.yaml`](../../infra/helm/meat-memory/values.yaml)

最小检查：

```bash
helm lint infra/helm/meat-memory
```

当前 chart 支持：

- app / worker 分镜像
- 复用已有 Secret
- 复用已有 PVC
- worker 按需启用

## 4. 发布维护者：源码 checkout 后打包

本节只适合已经克隆源码仓库、安装 Rust toolchain 并负责发布维护的人。普通安装用户不要运行这些命令。

本地打二进制包：

```bash
cargo build --release --locked --target x86_64-unknown-linux-gnu \
  -p memory-cli -p memory-app -p memory-worker

./docs/scripts/build-release-artifacts.sh \
  --target x86_64-unknown-linux-gnu \
  --asset-name meat-memory-linux-amd64
```

构建 skill bundle：

```bash
./docs/scripts/build-agent-skills-bundle.sh all ./dist/release
```

GitHub Actions release 流水线当前覆盖：

- 多平台二进制包
- GitHub Release 附件
- Agent skill bundle
- `ghcr.io/<owner>/meat-memory-app`
- `ghcr.io/<owner>/meat-memory-worker`

## 5. 相关文档

- [`usage-guide.md`](usage-guide.md)：源码仓库开发使用说明
- [`local-deploy-v1.md`](local-deploy-v1.md)：本地 Docker Compose 部署
- [`cloud-deploy-v1.md`](cloud-deploy-v1.md)：云端 Docker / Helm 部署
- [`systemd-deploy.md`](systemd-deploy.md)：单机 systemd 部署
- [`../agent-skills/README.md`](../agent-skills/README.md)：Agent Skill 交付维护
- [`../architecture-design/install-package-deploy-plan.md`](../architecture-design/install-package-deploy-plan.md)：安装、打包与部署完整方案
- [`../../packaging/npm/README.md`](../../packaging/npm/README.md)：npm 包装层说明
- [`../../packaging/homebrew/README.md`](../../packaging/homebrew/README.md)：Homebrew 发布方案
