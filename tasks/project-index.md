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

- 当前仓库已经从“纯文档仓库”推进到“工程基线 + 存储层 + kernel 主链路 + 基础可观测性落地”
- 已具备 Rust workspace、核心领域模型、PG/Markdown 存储层、`memory-kernel` remember/search/publish 编排、entity/relation 抽取、HTTP remember/search/health/metrics 路由、CLI remember/search/serve 入口、MCP remember/search/fetch_context/publish tool 层、内存版 sync oplog/merge baseline、基础配置、脚本、CI skeleton、Dockerfile、compose、本地 pgvector 开发库与 Git hooks
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
- 用途：定义 bind、logging、markdown、postgres、assets、sync、features
- 当前价值：`memory-app` 已从此文件读取配置

`scripts/`

- 角色：本地开发与验证入口
- 用途：环境验证、bootstrap、本地 pgvector 库启动与关闭
- 当前价值：`verify.sh`、`bootstrap.sh`、`dev-db-up.sh`、`dev-db-down.sh` 已可用

`compose.yaml`

- 角色：本地容器编排
- 用途：启动项目专用 `pgvector` 开发库
- 当前价值：已验证容器可启动且 `vector` 扩展可用

`Dockerfile`

- 角色：应用运行镜像骨架
- 用途：提供后续本地容器运行和 CI 构建基础
- 当前价值：已存在，尚待后续 release/workflow 完整接入

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
- HTTP `POST /api/v1/memories` 与 `POST /api/v1/context/search` 已可用
- HTTP `/healthz`、`/readyz`、`/livez`、`/metrics` 已可用
- `memory-cli` 已支持 `remember`、`search`、`serve`
- `memory-mcp` 已支持 tool listing、stdio message handler、HTTP `/mcp/tools`/`/mcp/tools/call`，以及 `memory.remember`、`memory.search`、`memory.fetch_context`、`memory.publish`
- `memory-sync` 已支持 `OplogEntry`/`SyncCursor`/`SyncBatch`/`ApplyBatchResult`、`append_oplog_entry`、`merge_ops` 与内存版 replication engine
- memory-observability 已提供结构化日志字段约定与检索/写入指标快照
- 最小 app/cli/worker 入口已可编译和运行
- docker compose 本地栈已运行并通过健康检查
- `.env.example`、`justfile`、pre-commit/commit-msg hooks 已补齐
- 本地 pgvector 目标库已可用

换句话说，当前项目已经从“想清楚”和“拆任务”的阶段，进入了“最小 remember/search 闭环已成型，可以开始做 richer policy/extract/publish/sync”的阶段。

## 4. 当前缺失的关键实现资产

以下资产目前缺失：

- `remember_media` 与多模态后台处理链路
- 持久化 sync engine、后台消费与 projection refresh job
- 更完整的 policy/redaction/review 流程
- 更完整的可观测性扩展（分布式 tracing、sync metrics、告警与 dashboard）
- 更完整的部署、发布、安全门禁与运维资产

这意味着后续重点已经不再是脚手架、基础存储或最小接口，而是把 `policy -> extract -> publish -> sync -> observability` 做成真实功能。

## 5. 推荐执行起点

建议执行顺序：

1. 完成 `MM-KER-003`，把媒体资产 ingest 和后台任务挂上
2. 完成 `MM-SYNC-004/005`，把 worker 消费与 projection refresh 接上
3. 继续扩展 `MM-OBS-004/005/007`，把 sync/tracing/alerting 补齐
4. 补 richer policy/redaction/review
5. 再进入发布、部署、安全与文档收尾

环境与脚手架阶段已经完成，当前最重要的是避免 kernel/HTTP 之外的 crate 长期停留在 placeholder 状态。

## 6. 建议的第一批可执行任务

最先启动的任务建议是：

- `MM-KER-003`
- `MM-SYNC-004`
- `MM-OBS-005`
- `MM-OBS-004`

这组任务完成后，项目就会从“已具备最小写入与检索闭环”进入“真正可发布、可抽取、可扩展的内核仓库”。

## 7. 当前风险与注意事项

- 当前仓库不是成熟代码仓库，任何后续自动化都应先围绕脚手架搭起来
- 当前大量 crate 仍是占位骨架，必须尽快替换默认模板实现
- `tasklist.md` 已较细，但仍然是执行规划，不等于具体代码实现
- 若后续目录结构发生变化，应同步更新本文件和 `task-log.md`
- `docs/meat-memory-scheme-v2.md` 应继续作为主设计依据，避免实现过程漂移回 `v1`

## 8. 下一步建议

建议下一步直接扩展当前可运行闭环：

1. 给 `memory-kernel` 增加 `remember_media`，推进 `MM-KER-003`。
2. 给 `memory-worker` 与 `memory-sync` 接上实际消费与 projection refresh，推进 `MM-SYNC-004/005`。
3. 把 observability 从单节点快照扩展到 tracing/alerting，推进 `MM-OBS-004`、`MM-OBS-005`、`MM-OBS-007`。
4. 继续补强 policy/redaction/review 与媒体写入，推进 `MM-POL-*`、`MM-KER-003`。
