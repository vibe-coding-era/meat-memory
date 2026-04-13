# V1 Release Notes

更新时间：2026-04-10

## 1. 发布信息

| 项目 | 内容 |
| --- | --- |
| 版本 | `0.1.0` |
| 发布阶段 | `V1` 封板版本 |
| 封板日期 | `2026-04-09` |
| 目标定位 | 纯自研、可自托管的长期 Memory 内核 |
| 技术栈 | Rust workspace + PostgreSQL + Markdown projection |
| 部署口径 | 本地独立部署、云端独立部署，混合部署预留接口 |

## 2. V1 交付范围

| 范围 | V1 结果 |
| --- | --- |
| 结构化主存 | 已支持 PostgreSQL 写入、检索、发布更新 |
| 本地投影 | 已支持 Markdown projection、rollup、roundtrip |
| 核心链路 | 已支持 `remember_text`、`search_context`、`publish_memory` |
| 知识图谱 | 已支持 entity / relation 抽取与 graph context baseline |
| Agent 接入 | 已支持 HTTP / CLI / MCP |
| 模型抽象 | 已支持 reasoning / extraction / vision / embedding 能力抽象 |
| 模型目录 | 已兼容 Gemini / Claude / ChatGPT / 千问 / 豆包 / MiniMax / GLM |
| Failover | 已支持 `primary + fallbacks` 自动切换与 `llm_notice` 提示 |
| 多模态 | V1 已支持文本 + 图片 |
| 中文优先 | 已支持中文抽取、检索、图谱样例、中文 caption |
| 部署 | 已支持 Compose 本地栈、Docker 镜像、Helm chart |
| 同步抽象 | 已提供 `sync/oplog/merge` baseline，为混合部署预留接口 |

## 3. 验收入口

| 入口 | 用途 |
| --- | --- |
| `cargo test --workspace --lib --bins --quiet` | 工作区主回归 |
| `./docs/scripts/v1-acceptance.sh` | V1 自动化验收入口 |
| `docker compose config --quiet` | 本地部署编排校验 |
| `helm lint infra/helm/meat-memory` | 云部署 chart 校验 |

## 4. 关键回归摘要

| 模块 | 已覆盖内容 |
| --- | --- |
| `memory-extract` | 中文关系抽取与图谱样例 |
| `memory-kernel` | 中文 remember / search / publish |
| `memory-http` | 中文文本流、Browser Console 首页 `/`、图片 failover `llm_notice` |
| `memory-mcp` | 中文 remember / search / publish 工具调用 |
| `memory-cli` | 中文 remember / search、图片 failover JSON 输出 |
| `tests/integration/v1-zh-acceptance.md` | 中文系统化验收语料入口 |

## 5. 使用入口

| 入口 | 说明 |
| --- | --- |
| [README.md](/Users/Rou/dev_projects/meat-memory/README.md) | 本地试跑、接口说明、V1 边界 |
| [docs/agent-integration-v1.md](/Users/Rou/dev_projects/meat-memory/docs/agent-integration-v1.md) | Agent 接入说明 |
| [docs/api/http-api-v1.md](/Users/Rou/dev_projects/meat-memory/docs/api/http-api-v1.md) | HTTP API |
| [docs/api/cli-v1.md](/Users/Rou/dev_projects/meat-memory/docs/api/cli-v1.md) | CLI 用法 |
| [docs/api/mcp-tools-v1.md](/Users/Rou/dev_projects/meat-memory/docs/api/mcp-tools-v1.md) | MCP 工具面 |
| [docs/runbook/local-deploy-v1.md](/Users/Rou/dev_projects/meat-memory/docs/runbook/local-deploy-v1.md) | 本地部署 |
| [docs/runbook/cloud-deploy-v1.md](/Users/Rou/dev_projects/meat-memory/docs/runbook/cloud-deploy-v1.md) | 云端部署 |
| [docs/runbook/v1-acceptance.md](/Users/Rou/dev_projects/meat-memory/docs/runbook/v1-acceptance.md) | V1 验收口径 |

## 6. 明确不在 V1 的范围

| 范围 | 归属版本 |
| --- | --- |
| 多团队 / 个人隔离与合并 | `V2` |
| 中英双语 | `V2` |
| 音频 / 视频 | `V3` |
| 完整混合部署复制执行 | `V2+` |

## 7. 当前结论

`V1` 已完成并封板。当前仓库已经具备：

- 可运行的文本 + 图片长期 Memory 最小闭环
- 面向 Agent 的 HTTP / CLI / MCP 接入面
- 中文优先的抽取、检索与图谱回归入口
- 本地独立部署与云端独立部署的最小交付包

后续主线转入 `V2`：团队/个人隔离、发布共享策略扩展，以及中英双语支持。
