# 产品功能结构说明书

## 1. 文档目标

本文档用于说明 `Meat Memory` 当前产品的功能边界、模块结构、用户可见能力以及主要运行链路，帮助研发、产品、运维和接入方快速理解系统全貌。

## 2. 产品定位

`Meat Memory` 是一个面向 Agent 与模型系统的长期记忆内核，核心目标是把“会话内信息”沉淀为“可持久、可检索、可跨端共享、可按 scope 治理”的 Memory 资产。

当前版本重点覆盖：

- 文本与图片记忆写入
- PostgreSQL + Markdown 双存储
- HTTP / CLI / MCP 多入口接入
- 中文优先与多模型路由
- V2 scope promotion / 同步基线 / 双语字段透传

## 3. 用户与使用角色

### 3.1 研发 / 本地开发者

主要诉求：

- 快速本地启动服务
- 调试 Memory 写入、搜索、发布和 promote
- 用 CLI / HTTP / MCP 接入 Agent

### 3.2 Agent / 上层编排系统

主要诉求：

- 通过标准接口写入长期记忆
- 根据 scope 拉取上下文
- 在不同 scope 之间 promote 记忆

### 3.3 运维 / 部署人员

主要诉求：

- 使用 Docker 或云配置部署服务
- 查看健康检查、指标和配置入口
- 固化测试、验收和安全检查流程

## 4. 顶层功能结构

### 4.1 记忆采集层

- 文本记忆写入：支持标题、正文、memory kind、visibility、sensitivity
- 图片记忆写入：支持图片字节落盘、vision caption、派生文本写入
- Artifact 构建：把输入先转为标准 artifact，再派生 memory

### 4.2 记忆存储层

- PostgreSQL 存储：结构化写入、关键词检索、版本化落库
- Markdown 存储：人类可读文档落盘与 scope 级目录组织
- 资产存储：图片等二进制内容按 `asset://` 规则引用

### 4.3 记忆理解层

- Entity / Relation 抽取
- 上下文图谱构建
- 中文优先抽取与检索语料支持
- 语言标记透传

### 4.4 记忆治理层

- Visibility 分级：private / project / team / organization
- Sensitivity 分级：public / internal / private / restricted
- Scope Promotion：从 user scope promote 到 project / team 等共享 scope
- Policy 判定：allow / review / deny
- Redaction：共享场景自动脱敏

### 4.5 接入层

- HTTP API：浏览器控制台、健康检查、Memory API、Context Search
- MCP：Agent 工具清单与工具调用
- CLI：本地脚本化接入与调试入口

### 4.6 运行与部署层

- 本地开发运行
- Docker Compose 运行
- 云端部署骨架
- Worker 后台进程
- CI / Release / Security 工作流

## 5. 功能树

## 5.1 写入域

- `remember text`
- `remember image`
- `artifact -> memory` 派生
- evidence link

## 5.2 检索域

- context search
- browse memories
- get memory
- 中文验收语料回归

## 5.3 发布与共享域

- publish visibility 升级
- scope promote
- owner_scope_id / published_from_scope_id 追踪
- candidate review 状态流转

## 5.4 模型与多模态域

- provider catalog
- capability routing
- primary + fallback 自动切换
- vision caption 与图片理解

## 5.5 同步域

- oplog entry
- pull / apply
- merge / conflict record
- file-backed replication state

## 6. 当前主要产品对象

### 6.1 Scope

代表记忆归属与访问边界，常见类型包括：

- user
- project
- team
- organization
- session

### 6.2 Artifact

原始采集对象，用来承载文本、图片、终端输出等输入内容。

### 6.3 Memory

系统的核心长期知识对象，带有：

- scope_id
- owner_scope_id
- published_from_scope_id
- visibility
- sensitivity
- memory_state
- language_code

### 6.4 Entity / Relation

用于构造知识图谱与上下文关系网络。

### 6.5 Sync Oplog

用于跨节点同步和冲突记录的操作日志对象。

## 7. 关键链路说明

### 7.1 文本写入链路

1. 接入层接收文本请求
2. Kernel 构建 Artifact 与 Memory
3. 抽取 entity / relation
4. 同时写入 PostgreSQL 与 Markdown
5. 返回 memory_id 与写入状态

### 7.2 图片写入链路

1. 接收图片字节和基础元数据
2. 资产存储落盘并生成 `asset_uri`
3. Vision 路由生成 caption / notice
4. 派生文本 Memory 并写入双存储
5. 返回图片资产与记忆结果

### 7.3 Context Search 链路

1. 上层输入 scope_id 与 query
2. Kernel 规范化搜索词
3. 存储层返回 Memory
4. Kernel 构建 entity / relation graph
5. 返回上下文包

### 7.4 Scope Promote 链路

1. 从源 scope 读取 Memory
2. Policy 评估目标 scope 与 visibility
3. 复制为新 Memory
4. 保留 owner_scope / published_from_scope
5. 必要时 redaction 并进入 `candidate`
6. 写入目标 scope

### 7.5 Sync 链路

1. 生成 oplog entry
2. 下游节点 pull
3. 节点 apply batch
4. 冲突写入 conflict record
5. 更新 sync status

## 8. 对外接口结构

### 8.1 HTTP

当前重点入口：

- `/api/v1/memories`
- `/api/v1/images`
- `/api/v1/context/search`
- `/api/v1/memories/promote`
- `/api/v1/meta`
- `/healthz` / `/readyz` / `/livez` / `/metrics`

### 8.2 MCP

当前重点工具：

- `memory.remember`
- `memory.search`
- `memory.fetch_context`
- `memory.publish`
- `memory.promote`

### 8.3 CLI

当前重点命令：

- `remember`
- `remember-image`
- `search`
- `serve`

## 9. 配置结构总览

### 9.1 运行配置

- `.env.example`
- `config/app.toml`
- `config/README.md`
- `config/app.local.toml`（由 TUI 生成，不建议提交真实私有配置）

### 9.2 工程配置

- `.cargo/config.toml`
- `rust-toolchain.toml`
- `clippy.toml`
- `rustfmt.toml`

### 9.3 自动化配置

- `.github/workflows/ci.yml`
- `.github/workflows/release.yml`
- `.github/workflows/security.yml`
- `compose.yaml`

## 10. 当前版本边界

当前仓库已完成 V1，并已补齐 V2 的关键能力基线，包括：

- scope-aware promotion
- 双语字段透传
- file-backed sync baseline
- 更强的集成测试与性能烟测
- 持久化测试报告体系

尚未完全展开的方向：

- 音频 / 视频 Memory
- 更大规模并发压测
- 完整远端同步协议
- 可执行 security suite

## 11. 推荐阅读路径

如果你第一次进入项目，推荐按下面顺序阅读：

1. `README.md`
2. `docs/product-feature-structure.md`
3. `docs/meat-memory-scheme-v2.md`
4. `docs/api/`
5. `docs/runbook/`
