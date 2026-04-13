# V2.5 Release Notes

更新时间：2026-04-13

## 1. 发布信息

| 项目 | 内容 |
| --- | --- |
| 版本 | `v2.5` |
| 发布阶段 | 安装、打包与部署封板版本 |
| 封板日期 | `2026-04-13` |
| 目标定位 | 把 V2.4 的短期 / 中期 Memory 能力收口为可安装、可打包、可部署的交付形态 |
| 技术栈 | Rust workspace + PostgreSQL + Markdown projection + HTTP / CLI / MCP |
| 部署口径 | Docker Compose、本地二进制、systemd、Docker 镜像、Helm/Kubernetes |

## 2. V2.5 交付范围

| 范围 | V2.5 结果 |
| --- | --- |
| 短期 Memory | 已完成 Agent Context 的 CLI / HTTP / MCP 主链路 |
| 中期 Memory | 已完成 Project Documents、source 多 key、本地文档同步与冲突报告 |
| Markdown projection | 已新增 project document projection，按 source 维度独立落盘 |
| 验收 | 已新增并实跑 V2.4 验收脚本和报告产物 |
| 预编译二进制 | 已补多平台 release tarball 与 sha256 打包脚本 |
| Agent Skill | 已补 `agent-skills-bundle.zip` 标准交付物 |
| npm | 已补可配置 releaseBaseUrl 的 npm 包装层骨架 |
| cargo install | 已验证 `cargo install --path crates/memory-cli --locked` 路径 |
| Homebrew | 已补 Formula 模板和发布流程说明 |
| Docker | 已拆分 `meat-memory-app` / `meat-memory-worker` 镜像 target 与命名约定 |
| Compose | 已统一本地 app / worker / pgvector 试跑入口与镜像变量 |
| systemd | 已新增 app / worker service 模板和单机部署 runbook |
| Helm | 已支持 app / worker 分镜像、现有 Secret / PVC 复用 |
| Release CI | 已补 tag 驱动的 release assets、skills bundle、Docker 镜像发布骨架 |
| 用户文档 | 已新增 `docs/runbook/install.md` 作为安装、打包、部署统一入口 |

## 3. 关键入口

| 入口 | 说明 |
| --- | --- |
| [`README.md`](../README.md) | 项目入口和快速开始 |
| [`docs/runbook/install.md`](runbook/install.md) | 面向用户的安装、打包与部署入口 |
| [`docs/runbook/systemd-deploy.md`](runbook/systemd-deploy.md) | 单机 Linux + systemd 部署说明 |
| [`docs/runbook/cloud-deploy-v1.md`](runbook/cloud-deploy-v1.md) | 云端 Docker / Helm 部署说明 |
| [`docs/architecture-design/install-package-deploy-plan.md`](architecture-design/install-package-deploy-plan.md) | 安装、打包与部署完整方案 |
| [`packaging/npm/README.md`](../packaging/npm/README.md) | npm 包装层说明 |
| [`packaging/homebrew/README.md`](../packaging/homebrew/README.md) | Homebrew 发布方案 |
| [`infra/systemd/README.md`](../infra/systemd/README.md) | systemd 模板说明 |
| [`infra/helm/meat-memory/values.yaml`](../infra/helm/meat-memory/values.yaml) | Helm values |

## 4. 验证摘要

本轮已执行或沿用的关键验证：

- `cargo install --path crates/memory-cli --locked --root /tmp/meat-memory-cargo-install-check`
- `memory-cli --help`
- `bash -n docs/scripts/build-release-artifacts.sh`
- `bash ./docs/scripts/build-agent-skills-bundle.sh all /tmp/meat-memory-release-check`
- `docker compose config --quiet`
- `helm lint infra/helm/meat-memory`
- `npm run check`，位于 `packaging/npm`

说明：

- Docker app / worker target 构建已进入正式 Rust release build 阶段，验证了 target 切分和 compose 引用链路。
- npm 和 Homebrew 当前是发布骨架与模板，未宣称已经发布到公共 registry / tap。

## 5. 后续范围

| 范围 | 说明 |
| --- | --- |
| npm 正式发布 | 等正式 GitHub release 下载域名、npm scope 和 publish token 确定后接入 |
| Homebrew 自动 bump | 等正式 tap 地址确定后接入 release workflow |
| V3 多模态 | 后续进入音频 / 视频 ingest、转写、关键帧和全多模态检索 |

## 6. 当前结论

`v2.5` 作为安装、打包与部署封板版本，已经把 V2.4 的核心能力整理成可分发、可部署、可交接的工程形态。

后续主线可以进入两条路径：

- 发布工程化：补齐真实 npm / Homebrew 发布凭证和自动化。
- 能力演进：进入 V3 音频 / 视频多模态。
