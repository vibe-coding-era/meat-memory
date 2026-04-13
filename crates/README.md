# Crates Index

`crates/` 是 Rust workspace 的核心实现区，按“入口层、内核层、领域层、基础设施层”拆分。

## 入口与进程

- `memory-app`：应用主进程，装配 HTTP、MCP、配置和观测
- `memory-worker`：后台工作进程
- `memory-cli`：命令行入口、初始化向导和运维命令

## 业务与内核

- `memory-kernel`：记忆主流程编排
- `memory-core`：核心服务抽象
- `memory-policy`：权限、发布和治理规则
- `memory-sync`：同步、合并和 oplog 能力
- `memory-extract`：抽取与转换

## 领域与存储

- `memory-domain`：领域对象、标识和生命周期
- `memory-store`：存储抽象 trait
- `memory-store-pg`：PostgreSQL 持久化实现
- `memory-store-md`：Markdown 投影实现

## 基础设施与适配

- `memory-http`：HTTP 协议适配层
- `memory-mcp`：MCP 协议适配层
- `memory-config`：配置加载与环境变量覆盖
- `memory-models`：模型 provider 与路由
- `memory-observability`：日志、trace、metrics
- `memory-assets`：图片等资产处理
- `memory-index`：索引相关能力预留与承载

如果你第一次读代码，建议先看 [`../docs/architecture/system-design.md`](../docs/architecture/system-design.md)。
