# Meat Memory 项目索引

更新时间：2026-04-05

索引范围：`/Users/Rou/dev_projects/meat-memory`

## 1. 当前目录结构

```text
.
├── .cargo/
├── .env.example
├── .githooks/
├── .github/
├── config/
├── crates/
├── docs/
│   ├── meat-memory-prompt.md
│   ├── meat-memory-scheme-v1.md
│   └── meat-memory-scheme-v2.md
├── infra/
├── scripts/
└── tasks/
    ├── project-index.md
    ├── task-log.md
    └── tasklist.md
├── tests/
├── Cargo.toml
├── CHANGELOG.md
├── Dockerfile
├── LICENSE
├── README.md
├── compose.yaml
├── justfile
└── rust-toolchain.toml
```

说明：

- 当前仓库已经从“纯文档仓库”推进到“工程基线 + 存储层 + kernel 主链路 + 基础可观测性 + V1 模型网关与图片理解基线 + 部署/文档/验收收口”。
- 已具备 Rust workspace、核心领域模型、PG/Markdown 存储层、`memory-kernel` remember/search/publish 编排、entity/relation 抽取、`memory-models` capability traits + provider/model/route registry + 本地 vision gateway、HTTP remember/search/health/metrics 路由、CLI remember/search/serve 入口、MCP remember/search/fetch_context/publish tool 层、内存版 sync oplog/merge baseline、基础配置、脚本、CI skeleton、Dockerfile、compose、本地 pgvector 开发库、Helm chart、V1 文档包、V1 验收脚本与 Git hooks。
- 当前最重要的有效输入变成了三类：`docs/` 下的设计文档、`tasks/` 下的执行文档、`crates/` 下的真实工程实现

## 2. 文件角色索引

### 2.1 `docs/`

`docs/meat-memory-prompt.md`

- 角色：原始需求输入
- 用途：记录产品目标、兼容范围、技术方向和约束
- 当前价值：作为所有方案与任务拆解的根需求源

`docs/meat-memory-scheme-v1.md`

- 角色：第一版总体方案
- 用途：定义长期 Memory 的总体方向、关键能力和边界
- 当前价值：作为高层架构与范围界定参考

`docs/meat-memory-scheme-v2.md`

- 角色：细化后的架构设计主文档
- 用途：定义对象模型、写入/检索链路、存储分层、同步、协议、Rust workspace 设计
- 当前价值：后续实现的主依据

### 2.2 `tasks/`

`tasks/tasklist.md`

- 角色：原子任务总清单
- 用途：从环境安装、仓库脚手架、内核实现到部署上线的执行路线图
- 当前价值：执行入口文档

`tasks/task-log.md`

- 角色：结构化执行日志
- 用途：记录索引、任务完成、决策、阻塞和下一步动作
- 当前价值：执行过程上下文主日志

`tasks/project-index.md`

- 角色：项目现状索引
- 用途：提供快速上下文、目录盘点、状态判断和开工入口
- 当前价值：执行前阅读入口

### 2.3 工程基线

`Cargo.toml`

- 角色：Rust workspace 根清单
- 用途：统一 members、共享依赖和 lint 策略
- 当前价值：workspace 已可 `cargo check`

`rust-toolchain.toml`

- 角色：Rust 工具链 pin
- 用途：统一团队编译版本与组件
- 当前价值：已固定到 `1.85.0`

`config/default.toml`

- 角色：默认运行配置
- 用途：定义 bind、logging、markdown、postgres、assets、models、sync、features
- 当前价值：`memory-app`、`memory-cli`、`memory-worker` 已从此文件读取并校验模型 registry

`scripts/`

- 角色：本地开发与验证入口
- 用途：环境验证、bootstrap、本地 pgvector 库启动与关闭、V1 验收执行
- 当前价值：`verify.sh`、`bootstrap.sh`、`dev-db-up.sh`、`dev-db-down.sh`、`v1-acceptance.sh` 已可用

`compose.yaml`

- 角色：本地容器编排
- 用途：启动项目专用 `pgvector`、`app`、`worker` 本地栈
- 当前价值：已补齐 Markdown/assets 持久卷，`docker compose config --quiet` 通过

`Dockerfile`

- 角色：应用运行镜像骨架
- 用途：提供本地容器运行和云部署镜像基础
- 当前价值：已修正 `memory-worker` 二进制复制链路，并通过 `docker build -t meat-memory:local-check .`

`infra/helm/meat-memory/`

- 角色：V1 云端独立部署包
- 用途：提供 app/service/pvc/config/secret 的最小 Helm 发布闭环
- 当前价值：`helm lint infra/helm/meat-memory` 已通过

## 3. 当前实现状态

当前状态判断：

- 需求已形成
- 高层方案已形成
- 细化设计已形成
- 执行任务清单已形成
- 执行日志机制已形成
- 工程仓库已初始化
- 核心领域模型已实现并有单测
- PostgreSQL store 已实现并有集成测试
- Markdown store 已实现并有 roundtrip 测试
- memory-kernel 已实现最小 dual-write remember/search 编排
- memory-policy 已提供最小 write policy allow/review/deny 规则
- memory-extract 已提供 distill/entity/relation 抽取与独立测试
- memory-kernel 已实现 publish_memory 与 PG+Markdown remember→search→publish 集成测试
- HTTP `POST /api/v1/memories`、`POST /api/v1/images` 与 `POST /api/v1/context/search` 已可用
- HTTP `/healthz`、`/readyz`、`/livez`、`/metrics` 已可用
- `memory-cli` 已支持 `remember`、`search`、`serve`
- `memory-mcp` 已支持 tool listing、stdio message handler、HTTP `/mcp/tools`/`/mcp/tools/call`，以及 `memory.remember`、`memory.search`、`memory.fetch_context`、`memory.publish`
- `memory-app` 与 `memory-cli serve` 在开启 `enable_mcp` 时已真实挂出 MCP HTTP 路由
- `memory-models` 已支持 reasoning / extraction / vision / embedding 抽象、provider/model descriptor、fallback route registry
- `memory-models` 已支持本地 image profile 解析、vision gateway 和中文优先 caption 生成
- `memory-config` 已接入 Gemini / Claude / ChatGPT / 千问 / 豆包 / Minimax / GLM 的 provider catalog 与默认路由配置
- `memory-assets` 已支持 sha256 寻址、本地文件存储、分类目录、asset metadata 与回读测试
- `memory-kernel` 已支持 `remember_image`
- HTTP 已支持 `POST /api/v1/images`，CLI 已支持 `remember-image`
- 图片写入时已自动生成最小 vision caption 与结构化派生文本，并写回 Artifact/Memory
- `memory-sync` 已支持 `OplogEntry`/`SyncCursor`/`SyncBatch`/`ApplyBatchResult`、`append_oplog_entry`、`merge_ops` 与内存版 replication engine
- memory-observability 已提供结构化日志字段约定与检索/写入指标快照
- 最小 app/cli/worker 入口已可编译、运行并在启动期校验模型路由配置
- docker compose 本地栈已具备 app/worker/pgvector 拓扑、持久卷和配置校验
- `.env.example`、`justfile`、pre-commit/commit-msg hooks 已补齐
- 本地 pgvector 目标库已可用
- V1 用户文档包已补齐：Agent 接入、HTTP/MCP/CLI、local/cloud runbook、acceptance
- V1 验收脚本 `./scripts/v1-acceptance.sh` 已补齐并通过

换句话说，当前项目已经从“想清楚”和“拆任务”的阶段，进入了“文本 + 图片最小闭环已成型，可以开始做 Agent 接入、部署和验收收口”的阶段。

## 4. 当前缺失的关键实现资产

以下资产目前仍是主要缺口：

- `V1-ZH-001`：中文优先的系统化验收语料和回归集
- V2 范围内的多团队/个人隔离与中英双语扩展

这意味着项目已经从“把 V1 做成可交付”推进到“补齐 V1 中文验收集，并准备 V2”的阶段。

## 5. 推荐执行起点

建议执行顺序：

1. 完成 `V1-ZH-001`，把中文语料与回归入口系统化
2. 固化 V1 版本提交、标签和 release notes
3. 准备 V2 的 scope 与多语言扩展

## 6. 建议的第一批可执行任务

最先启动的任务建议是：

- `V1-ZH-001`
- V1 提交与版本封板
- V2 规划准备

当前项目已经处在“V1 可交付版本”的状态。

## 7. 当前风险与注意事项

- 当前图片 caption 已可自动生成，但尚未接入真实外部视觉 SDK，V1 仍属于“本地最小可运行 + 可切真实 SDK”的形态
- `tasklist.md` 已切到版本视图，后续执行与汇报应优先使用 `V1-* / V2-* / V3-*` 编号
- 若后续目录结构发生变化，应同步更新本文件和 `task-log.md`
- `docs/meat-memory-scheme-v2.md` 应继续作为主设计依据，避免实现过程漂移回旧的 `MM-*` 粒度规划
- `.cargo/config.toml` 已从“全局固定 `/usr/bin/clang`”收敛到“仅 Apple target 固定 clang”，避免 Linux 容器构建失败

## 8. 下一步建议

建议下一步直接扩展当前可运行闭环：

1. 做 `V1-ZH-001`，把中文样例、中文检索和中文图谱回归集补齐。
2. 固化当前 V1 交付版本并准备提交。
