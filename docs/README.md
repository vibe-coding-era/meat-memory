# 文档总览

`docs/` 是仓库的主文档区，建议按下面顺序阅读：

1. 先看 [`../README.md`](../README.md) 了解项目定位和快速入口。
2. 再看 [`runbook/usage-guide.md`](runbook/usage-guide.md) 熟悉启动、配置和常见命令。
3. 然后看 [`architecture/system-design.md`](architecture/system-design.md) 理解模块分层和数据流。

## 目录导航

- [`runbook/README.md`](runbook/README.md)：部署、启动、验收和排障入口
- [`architecture/README.md`](architecture/README.md)：架构文档导航
- [`api/README.md`](api/README.md)：CLI、HTTP、MCP 文档入口
- [`agent-skills/README.md`](agent-skills/README.md)：Agent Skill 模板、导出和接入说明
- [`default/README.md`](default/README.md)：默认 Markdown projection 与示例内容说明
- [`tasks/README.md`](tasks/README.md)：项目任务与执行上下文入口
- [`reports/README.md`](reports/README.md)：分析型报告入口
- [`tasks/project-index.md`](tasks/project-index.md)：任务索引与执行上下文

## 核心文档

- `meat-memory-prompt.md`：原始需求与产品背景
- `product-feature-structure.md`：功能结构、对象模型和能力拆解
- `agent-integration-v1.md`：面向 Agent 平台的接入说明
- `release-notes-v1.md`：V1 发布边界与延期范围

## 架构演进文档

- `meat-memory-scheme-v1.md`：V1 高层方案
- `meat-memory-scheme-v2.md`：V2 详细架构方案
- `meat-memory-scheme-v2_2.md`：key、权限隔离与索引设计
- `meat-memory-scheme-v2_3.md`：安全边界与专项验证
- `meat-memory-scheme-v2_4.md`：短期上下文、项目文档同步和 source 多 key
- `meat-memory-scheme-v3.md`：当前实现基线和下一阶段规划

## 建议维护方式

- 对外使用方式变化时，优先更新 `README.md` 与 `runbook/usage-guide.md`
- 模块边界变化时，优先更新 `architecture/system-design.md`
- 新增命令或协议时，补充 `api/` 下对应文档
