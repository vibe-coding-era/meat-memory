# 安装、打包与部署完整方案

本文档给出 `Meat Memory` 的安装、打包和部署统一方案，目标是让项目同时支持开发者安装、自托管部署、容器化交付和后续自动发布。

## 1. 目标

本方案解决三类问题：

- 用户如何安装：覆盖 `npm`、`cargo install`、预编译二进制、Homebrew、Docker
- 用户安装后如何获取可直接用于各类 Agent 的 skills
- 团队如何打包：统一 release 产物、版本号、命名规则和发布流程
- 系统如何部署：支持本地开发、单机部署、容器部署和 Kubernetes

## 2. 总体原则

采用“一套核心二进制，多种分发渠道”的策略：

- 核心实现仍然在 Rust workspace 中维护
- 安装渠道只做分发和轻包装，不重复实现业务逻辑
- Agent Skill 作为安装后的标准交付物，而不是额外文档附件
- 配置模型统一使用 `config/app.toml + MEAT_MEMORY_*`
- 发布流程统一由 Git tag 驱动

## 3. 安装方案

### 3.1 支持渠道

建议优先支持以下安装方式：

1. `npm`
2. `cargo install`
3. GitHub Releases 预编译二进制
4. Homebrew
5. Docker

无论用户通过哪种方式安装，安装完成后都应能拿到：

- `memory-cli`
- 面向主流 Agent 的 skill bundle
- 一份最小安装后指引，说明如何导入 skill 和接入 MCP

### 3.2 各渠道定位

#### `npm`

适用人群：

- Node / 前端 / Agent 用户
- 希望通过 `npx` 或全局命令快速安装的人

建议形式：

- 包名：`meat-memory` 或 `@meat-memory/cli`
- 安装命令：

```bash
npm install -g meat-memory
meat-memory --help
```

实现建议：

- npm 包只做平台分发
- 安装时按 OS/Arch 选择预编译二进制
- 可选提供 `npx meat-memory` 使用方式
- 安装后提供 `meat-memory skills export` 或等价命令导出 Agent skill

当前仓库的 npm 包装层骨架位于：

- `packaging/npm/package.json`
- `packaging/npm/bin/meat-memory.js`
- `packaging/npm/scripts/install.js`

正式 release 下载域名确定前，该包装层只作为可检查骨架，不作为已发布安装渠道。

#### `cargo install`

适用人群：

- Rust 开发者
- 需要源码级调试和本地构建的人

安装命令：

```bash
cargo install memory-cli
memory-cli --help
```

安装后推荐：

```bash
memory-cli skills export --target all --output-dir ./dist/agent-skills --force
```

#### GitHub Releases

适用人群：

- 不依赖 Node / Rust 的普通用户
- CI 或企业内部下载分发

建议支持平台：

- macOS `arm64`
- macOS `x86_64`
- Linux `x86_64`
- Linux `arm64`

建议产物命名：

```text
meat-memory-darwin-arm64.tar.gz
meat-memory-darwin-amd64.tar.gz
meat-memory-linux-amd64.tar.gz
meat-memory-linux-arm64.tar.gz
```

同时建议为 release 附带：

- `agent-skills-bundle.zip`
- 安装后 `README`

#### Homebrew

适用人群：

- macOS 开发者
- 希望系统级管理命令的用户

安装命令：

```bash
brew install <tap>/meat-memory
meat-memory --help
```

安装后推荐：

```bash
meat-memory skills export --target all --output-dir ./dist/agent-skills --force
```

当前仓库的 Homebrew 发布材料位于：

- `packaging/homebrew/README.md`
- `packaging/homebrew/meat-memory.rb.template`

正式 tap 地址和 release 下载域名确定前，Homebrew 仍只作为发布模板，不作为已发布安装渠道。

#### Docker

适用人群：

- 不想安装 Rust / Node 的用户
- 希望直接运行服务栈的团队

示例：

```bash
docker run --rm meat-memory-app:latest --help
docker compose up -d
```

对于 Docker 用户，也应提供：

- 容器外可执行的 skill 导出命令
- 或在镜像文档中说明如何通过 `memory-cli skills export` 生成 skill bundle

### 3.3 安装后 Agent Skill 交付

安装完成并不等于用户就能直接接入 Agent。为了让 Codex、Claude Code / TRAE / Qoder、执行型 Agent 立即可用，建议把 skill 交付作为安装闭环的一部分。

#### 目标

无论安装渠道是什么，用户都应该能在安装后完成下面这件事：

```bash
memory-cli skills export --target all --output-dir ./dist/agent-skills --force
```

或：

```bash
./docs/scripts/export-agent-skills.sh all ./dist/agent-skills
```

#### 建议支持的 skill 目标

- `codex`
- `claude-code`
- `execution-agent`
- `all`

#### 建议交付形式

安装渠道至少要满足下面一种：

1. 安装完成后本机可直接运行 `memory-cli skills export`
2. Release 附带单独的 `agent-skills-bundle.zip`
3. npm 包安装后可通过 postinstall 或命令入口生成 skill bundle

#### 安装后说明应包含

- 如何检查本地配置：`memory-cli config check`
- 如何检查 MCP：`memory-cli mcp info`
- 如何导出 skill：`memory-cli skills export ...`
- 如何把 skill 复制到目标 Agent 平台目录

#### 示例

```bash
memory-cli config check
memory-cli mcp info
memory-cli skills export --target all --output-dir ./dist/agent-skills --force
find ./dist/agent-skills -maxdepth 2 -type f | sort
```

## 4. 打包方案

### 4.1 核心产物

建议统一打包三类核心二进制：

- `memory-cli`
- `memory-app`
- `memory-worker`

### 4.2 Release 产物设计

每次发布建议产出：

- 多平台 CLI 二进制压缩包
- 服务端二进制压缩包
- Docker 镜像
- npm 包
- Agent skill bundle
- Homebrew Formula 更新信息

### 4.3 二进制包内容

建议 release tarball 包含：

- `memory-cli`
- `memory-app`
- `memory-worker`
- `config/app.toml` 示例
- `README` 或安装说明
- `docs/agent-skills/` 或导出的 skill bundle 入口说明

### 4.4 npm 打包设计

建议目录：

```text
packaging/npm/
```

职责：

- 维护 npm `package.json`
- 处理安装期平台识别
- 下载或选择正确的预编译二进制
- 暴露统一的 `meat-memory` 命令
- 提供导出 Agent skill 的命令入口

### 4.5 Docker 打包设计

建议至少提供：

- `meat-memory-app`
- `meat-memory-worker`

当前建议使用同一个 `Dockerfile` 的两个 build target：

- `app-runtime`
- `worker-runtime`

并统一镜像命名：

- `meat-memory-app:<tag>`
- `meat-memory-worker:<tag>`

本地 compose 默认使用 `:local`，release/云端与 Git tag 保持一致，例如 `:v0.4.0`。

### 4.6 版本策略

建议所有分发渠道共用同一语义化版本号，例如：

```text
v0.4.0
```

对应：

- Git tag：`v0.4.0`
- npm version：`0.4.0`
- Docker tag：`0.4.0` 与 `latest`
- Homebrew formula：`0.4.0`

## 5. 部署方案

### 5.1 本地开发部署

适用场景：

- 本地开发
- 联调 HTTP / MCP / worker

推荐方式：

```bash
./docs/scripts/dev-up.sh
./docs/scripts/dev-down.sh
```

本地 compose 默认会构建：

- `meat-memory-app:local`
- `meat-memory-worker:local`

如需覆盖，可在 `.env` 中设置：

- `MEAT_MEMORY_APP_IMAGE`
- `MEAT_MEMORY_WORKER_IMAGE`
- `MEAT_MEMORY_IMAGE_TAG`

### 5.2 单机部署

适用场景：

- 一台 Linux 主机自托管
- 中小团队内部服务

推荐组件：

- `memory-app` 使用 `systemd`
- `memory-worker` 使用 `systemd`
- PostgreSQL 可本机或外置

当前仓库建议直接复用：

- `infra/systemd/meat-memory-app.service`
- `infra/systemd/meat-memory-worker.service`
- `infra/systemd/meat-memory.env.example`
- `docs/runbook/systemd-deploy.md`

### 5.3 容器部署

适用场景：

- 需要 Docker 运行时
- 希望通过 `compose` 快速交付

推荐方式：

- 本地和中小规模生产环境优先 `docker compose`
- 配置通过环境变量覆盖

### 5.4 Kubernetes / Helm 部署

适用场景：

- 云端正式环境
- 需要滚动更新、Secret、PVC、Service 编排

建议保留并扩展：

- `infra/helm/meat-memory/`

建议资源：

- `Deployment`
- `Service`
- `ConfigMap`
- `Secret`
- `PVC`

当前 chart 已收口的约定：

- app 与 worker 支持分开镜像仓库：`image.repository` / `workerImage.repository`
- 支持复用已有 Secret：`secrets.existingSecretName`
- 支持复用已有 PVC：`persistence.existingClaim`
- worker 默认关闭，按需通过 `worker.enabled=true` 开启

## 6. 配置策略

安装、打包、部署三者都应共用同一套配置模型：

- 默认文件：`config/app.toml`
- 环境变量覆盖：`MEAT_MEMORY_*`

关键变量：

- `MEAT_MEMORY_CONFIG`
- `MEAT_MEMORY_SERVER_BIND`
- `MEAT_MEMORY_DATABASE_URL`
- `MEAT_MEMORY_MARKDOWN_ROOT`
- `MEAT_MEMORY_ASSETS_ROOT`
- `MEAT_MEMORY_ENABLE_MCP`

## 7. 仓库建议结构

建议中期整理为：

```text
docs/
  architecture-design/
  scripts/
  runbook/
packaging/
  npm/
  homebrew/
  release/
infra/
  helm/
compose.yaml
```

说明：

- `docs/`：文档和脚本入口
- `packaging/`：安装与分发材料
- `infra/`：部署骨架

## 8. 发布流水线建议

建议发布顺序：

1. 打 Git tag
2. CI 构建多平台二进制
3. 发布 GitHub Releases
4. 生成并上传 Agent skill bundle
5. 构建并推送 `meat-memory-app` / `meat-memory-worker` Docker 镜像
6. 发布 npm 包
7. 更新 Homebrew Formula
8. 更新安装文档

当前已落地：

- Git tag 触发 release workflow
- 多平台二进制 tarball + checksum
- GitHub Release 附件
- `agent-skills-bundle.zip`
- Docker 镜像发布骨架

当前仍待后续接入：

- npm publish
- Homebrew formula 自动更新

## 9. 示例

### 例子 1：普通用户安装 CLI

```bash
npm install -g meat-memory
meat-memory --help
```

安装后导出 Agent skill：

```bash
meat-memory skills export --target all --output-dir ./dist/agent-skills --force
```

### 例子 2：Rust 开发者安装

```bash
cargo install memory-cli
memory-cli --help
```

安装后导出 Codex skill：

```bash
memory-cli skills export --target codex --output-dir ./dist/codex-skills --force
```

### 例子 3：本地起完整服务

```bash
cp .env.example .env
./docs/scripts/dev-up.sh
curl http://127.0.0.1:8080/healthz
```

### 例子 4：云端发布流程

```text
git tag v0.4.0
git push origin v0.4.0
CI 构建 release 二进制
CI 推送 Docker 镜像
CI 发布 npm
CI 更新 Homebrew
```

## 10. 推荐优先级

建议实施优先级：

1. GitHub Releases 预编译二进制
2. npm 安装
3. 安装后 Agent skill 交付闭环
4. cargo install 路径收口
5. Docker 镜像规范化
6. Homebrew
7. systemd 单机部署模板
8. Helm 完善

原因：

- Release 二进制是其他安装渠道的基础
- npm 覆盖最广
- Agent 用户安装后需要立刻拿到 skill，不能再手工二次摸索
- cargo install 成本最低
- Docker 和 Helm 属于部署主链路
