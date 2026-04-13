# 架构设计

本目录作为技术文档中的“架构设计”总入口，负责统一收纳系统定位、模块边界、协议设计、部署形态和版本演进方案。

## 怎么使用

如果你是第一次接手这个项目，建议先读：

1. `system-design.md`，先建立整体分层认知
2. `product-feature-structure.md`，再看对象模型和功能边界
3. `meat-memory-scheme-v3.md`，最后补版本演进上下文

## 适合什么时候看

- 想搞清楚各个 crate 为什么这样拆
- 想知道 CLI / HTTP / MCP 如何共用同一套内核
- 想了解 PostgreSQL、Markdown、Assets 的协作关系
- 想回顾 V1 到 V3 的设计演进

## 推荐阅读顺序

1. [`../architecture/system-design.md`](../architecture/system-design.md)：当前实现基线的系统设计
2. [`../product-feature-structure.md`](../product-feature-structure.md)：功能结构、对象模型和链路边界
3. [`../meat-memory-scheme-v3.md`](../meat-memory-scheme-v3.md)：当前版本演进基线

## 文档分组

### 系统基线

- [`../architecture/system-design.md`](../architecture/system-design.md)：当前最适合作为维护者入口的系统架构设计
- [`../product-feature-structure.md`](../product-feature-structure.md)：功能结构、对象模型和模块能力拆解
- [`../default/README.md`](../default/README.md)：默认 Markdown projection 的结构说明

### 架构演进

- [`../meat-memory-scheme-v1.md`](../meat-memory-scheme-v1.md)：V1 高层架构方案
- [`../meat-memory-scheme-v2.md`](../meat-memory-scheme-v2.md)：V2 详细架构设计
- [`../meat-memory-scheme-v2_2.md`](../meat-memory-scheme-v2_2.md)：key、权限隔离和索引设计
- [`../meat-memory-scheme-v2_3.md`](../meat-memory-scheme-v2_3.md)：安全边界与专项收口
- [`../meat-memory-scheme-v2_4.md`](../meat-memory-scheme-v2_4.md)：短期上下文、项目文档同步和 source 多 key
- [`../meat-memory-scheme-v3.md`](../meat-memory-scheme-v3.md)：当前实现基线与下一阶段规划

### 协议与接入设计

- [`../api/README.md`](../api/README.md)：CLI、HTTP、MCP 接口索引
- [`../agent-integration-v1.md`](../agent-integration-v1.md)：面向 Agent 平台的接入说明
- [`../agent-skills/README.md`](../agent-skills/README.md)：Agent Skill 模板和接入方式

### 部署与运行设计

- [`../runbook/install.md`](../runbook/install.md)：面向用户的安装、打包与部署入口
- [`../runbook/local-deploy-v1.md`](../runbook/local-deploy-v1.md)：本地部署方案
- [`../runbook/cloud-deploy-v1.md`](../runbook/cloud-deploy-v1.md)：云端部署骨架
- [`../runbook/systemd-deploy.md`](../runbook/systemd-deploy.md)：单机 systemd 部署
- [`../runbook/v2_1-quickstart.md`](../runbook/v2_1-quickstart.md)：安装后快速配置和接入路径
- [`install-package-deploy-plan.md`](install-package-deploy-plan.md)：安装、打包与部署完整方案

## 例子

### 例子 1：第一次读架构

按这个顺序看最省力：

- [`../architecture/system-design.md`](../architecture/system-design.md)
- [`../product-feature-structure.md`](../product-feature-structure.md)
- [`../meat-memory-scheme-v3.md`](../meat-memory-scheme-v3.md)

### 例子 2：只关心接入方式

如果你要接入 Agent、CLI 或 HTTP，可以优先看：

- [`../api/README.md`](../api/README.md)
- [`../agent-integration-v1.md`](../agent-integration-v1.md)
- [`../agent-skills/README.md`](../agent-skills/README.md)

### 例子 3：只关心部署

如果你要本地起服务或准备云端部署，可以优先看：

- [`../runbook/local-deploy-v1.md`](../runbook/local-deploy-v1.md)
- [`../runbook/cloud-deploy-v1.md`](../runbook/cloud-deploy-v1.md)
- [`../runbook/systemd-deploy.md`](../runbook/systemd-deploy.md)
- [`../runbook/install.md`](../runbook/install.md)
- [`../runbook/v2_1-quickstart.md`](../runbook/v2_1-quickstart.md)
- [`install-package-deploy-plan.md`](install-package-deploy-plan.md)

## 维护约定

- 架构边界变化时，优先更新 `system-design.md`
- 版本方案变化时，优先更新 `meat-memory-scheme-v*.md`
- 新增对外协议或接入形态时，同时补 `api/` 或 `agent-*` 相关文档
