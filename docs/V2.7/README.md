# Meat Memory V2.7 生命周期系统方案

状态：Planning Draft

更新时间：2026-04-17

## 1. 版本目标

V2.7 的目标不是再发明一套全新的 Memory 系统，而是把已经存在的三层记忆骨架补成一个可演进、可治理、可解释的生命周期系统。

一句话目标：

把当前已经存在的短期 `Agent Context`、中期 `Project Documents / Memory Source`、长期 `Memory`，升级为统一协作、统一治理、统一召回的记忆生命周期系统。

## 2. 为什么是 V2.7

当前仓库已经具备以下现实基础：

- 短期记忆：`AgentContext`
- 中期记忆：`ProjectDocument`、`MemorySource`
- 长期记忆：`Memory`
- 基础理解：Entity / Relation 抽取、图谱增强上下文
- 基础治理：scope、visibility、sensitivity、publish、promote、candidate
- 基础召回：search / fetch_context、keyword + vector + graph expansion

但当前系统仍有明显缺口：

- 三层记忆分别存在，但缺少统一 schema 和生命周期语义
- 理解、压缩、召回、演进、遗忘更多停留在能力碎片或规划稿
- 对外文档对现状与目标的分界不够清晰
- 用户对“为什么记住、为什么召回、为什么不召回”仍缺少解释

因此 V2.7 的定位是：

- 不进入音频 / 视频全多模态
- 不推翻 V2.4-V2.6 已完成的结构
- 先把三层记忆骨架补成稳定、可扩展的生命周期底座

## 3. 版本边界

### 3.1 V2.7 要做

- 统一三层记忆的产品叙事和对象关系
- 建立 lifecycle-aware 的 Memory Schema
- 建立理解、压缩、召回、演进、治理的最小闭环
- 给出用户可控、可解释、可审计的治理入口设计
- 为后续 V3 多模态保留兼容的数据结构和策略接口

### 3.2 V2.7 不做

- 不把重点放在音频 / 视频 ingest
- 不追求复杂 GUI
- 不一开始做全自动高风险记忆更新
- 不为了新能力推倒当前 `AgentContext / ProjectDocument / Memory` 结构

## 4. 核心判断

V2.7 的设计必须建立在一个清晰判断上：

- 当前系统不是“只有长期记忆库”
- 当前系统已经有三层记忆骨架
- 真正缺的是统一生命周期，而不是更多入口

换句话说，V2.7 的核心不是“让系统更会存”，而是“让系统知道什么时候该写、写成什么类型、什么时候该召回、什么时候该降权、什么时候该归档、为什么这么做”。

## 5. 文档结构

- [`V2.7.1-current-capability-gap.md`](./V2.7.1-current-capability-gap.md)：现有能力与目标能力对比
- [`V2.7.2-lifecycle-schema.md`](./V2.7.2-lifecycle-schema.md)：统一记忆模型与生命周期设计
- [`V2.7.3-recall-governance.md`](./V2.7.3-recall-governance.md)：召回、解释、遗忘、治理设计
- [`V2.7.4-delivery-plan.md`](./V2.7.4-delivery-plan.md)：实施路线、任务分解、验收指标
- [`V2.7.5-surface-parity.md`](./V2.7.5-surface-parity.md)：MCP / CLI / HTTP API / TUI 接入面能力与参数一致性对比
- [`V2.7.6-production-quality-gate.md`](./V2.7.6-production-quality-gate.md)：商用生产门禁、严格验收和覆盖率阈值

## 6. 总体方案摘要

V2.7 将系统能力分成五个层次：

| 层 | 模块 | 职责 |
|---|---|---|
| 接入层 | HTTP / CLI / MCP / Skills | 暴露写入、召回、治理、审计能力 |
| 编排层 | Lifecycle Orchestrator | 决定何时理解、压缩、召回、更新、归档 |
| 智能层 | Understanding / Compression / Recall / Evolution / Governance | 执行分类、摘要、排序、冲突检测、治理判断 |
| 存储层 | AgentContext / ProjectDocument / Memory / Relation / Audit | 保存三层记忆、关系、状态、审计事件 |
| 策略层 | Scope / Privacy / Lifecycle / Recall Budget Policy | 统一限制作用域、敏感性、生命周期与上下文预算 |

## 7. V2.7 的交付结果

如果 V2.7 完成，系统应达到以下状态：

- Agent 能明确区分短期、中期、长期记忆，而不是把它们当成分散能力
- 记忆对象带有类型、来源、状态、置信度、生命周期信息
- 长任务结束后能生成结构化摘要，而不是仅保存原始过程
- 召回结果能解释“为什么被召回”
- 被遗忘、过期、敏感、已替代的内容不会继续污染召回
- 用户和维护者能查看、变更、归档、遗忘关键记忆

## 8. 与后续版本关系

V2.7 是 V3 多模态之前的生命周期底座版本。

推荐顺序：

1. 先完成 V2.7，补齐生命周期系统
2. 再进入 V3，把音频 / 视频接入这个统一生命周期底座

如果直接跳过 V2.7 进入多模态，系统会扩大“存得更多但治理更弱”的问题。
