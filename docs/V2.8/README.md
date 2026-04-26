# Meat Memory V2.8 用户参与式记忆治理方案

状态：Accepted

更新时间：2026-04-24

## 1. 版本目标

V2.8 的目标不是再扩一层新记忆，而是在 V2.7 生命周期底座之上，把“记忆会不会被合并、如何演进、何时遗忘、如何恢复、按什么方式提炼”变成用户可参与、可确认、可回滚的治理闭环。

一句话目标：

把当前的 lifecycle-aware memory system，升级为 proposal-first、versioned、user-steerable 的协作记忆系统。

## 2. 为什么是 V2.8

V2.7 已经补齐了：

- 三层记忆统一 record 视图
- recall guard / explanation
- forget / restore / audit / report

但当前系统仍缺少四个关键能力：

- 无法稳定回答“这是不是同一个项目”
- 无法区分“重复”“可合并”“替代”“冲突”
- 治理动作虽然存在，但缺少用户确认、版本时间线和恢复模型
- 记忆提炼过程还不能让用户带入自己的 prompt 和偏好

因此 V2.8 的定位不是推翻 V2.7，而是把 V2.7 的生命周期能力补成“用户参与式治理层”。

## 3. 版本边界

### 3.1 V2.8 要做

- 定义同项目、同内容、替代、冲突的判定模型
- 引入 `proposal-first` 的治理流程
- 把关键治理动作改为可审阅、可批准、可拒绝、可恢复
- 在现有 `memory_versions` 和 lifecycle audit 之上补齐时间线与回滚语义
- 引入用户可配置的记忆提炼 `Distillation Profile`
- 为 HTTP / CLI / MCP 设计一致的 review、timeline、rollback、profile 入口

### 3.2 V2.8 不做

- 不直接进入完整知识图谱产品化页面
- 不默认做高风险自动覆盖或自动 hard delete
- 不为了 dedupe/merge 引入过大的通用规则引擎
- 不在 V2.8 做完整分支式版本系统
- 不把用户自定义 prompt 变成绕过事实性约束的后门

## 4. 核心原则

- `proposal first`：除明确低风险场景外，系统先提案，再执行。
- `exact duplicate auto, semantic change review`：只有精确重复才能静默处理。
- `project identity before content merge`：先判断是不是同一项目，再判断是不是同一内容。
- `snapshot + relation + audit`：版本、关系、审计三条线都要保留。
- `prompt can steer distillation, not rewrite facts`：用户 prompt 只影响提炼，不影响来源和事实边界。

## 5. 文档结构

- [`V2.8.1-identity-and-merge.md`](./V2.8.1-identity-and-merge.md)：同项目 / 同内容判定与 merge 策略
- [`V2.8.2-proposal-review.md`](./V2.8.2-proposal-review.md)：proposal、review queue 与用户确认流程
- [`V2.8.3-versioning-recovery.md`](./V2.8.3-versioning-recovery.md)：版本、时间线、恢复与关系模型
- [`V2.8.4-distillation-profiles.md`](./V2.8.4-distillation-profiles.md)：用户可配置的记忆提炼模型
- [`V2.8.5-technical-architecture.md`](./V2.8.5-technical-architecture.md)：模块、存储、接口与迁移设计
- [`V2.8.6-delivery-plan.md`](./V2.8.6-delivery-plan.md)：里程碑、实施路线、测试与风险控制
- [`V2.8.7-tasklist.md`](./V2.8.7-tasklist.md)：V2.8 版本化任务清单
- [`V2.8.8-acceptance.md`](./V2.8.8-acceptance.md)：验收目标、场景与报告要求

## 6. 与 V2.7 / V3 的关系

V2.8 与相邻版本的边界建议如下：

| 版本 | 重点 | 角色 |
|---|---|---|
| V2.7 | lifecycle schema、recall guard、forget/restore、audit | 立底座 |
| V2.8 | proposal、review、version、rollback、user prompt distillation | 立治理闭环 |
| V3.4 | 项目认知网络、relations、timeline 深化 | 扩展演进网络 |
| V3.5 | 大规模治理任务、隐私维护、健康维护作业 | 扩展长期运营 |

V2.8 最重要的工程判断是：

先把“关键动作必须经人确认且可恢复”坐实，再做更复杂的自动冲突解决和知识网络扩张。

## 7. 总体方案摘要

V2.8 将新增五个核心能力层：

| 层 | 模块 | 职责 |
|---|---|---|
| 识别层 | `ProjectIdentityResolver` | 判断输入属于哪个项目边界 |
| 判定层 | `RelationshipClassifier` | 判断 duplicate / merge / supersede / conflict / unrelated |
| 治理层 | `ProposalOrchestrator` | 产出 proposal、分配 review level、驱动审批 |
| 版本层 | `VersionManager` / `TimelineQueryService` | 管理 snapshot、关系、rollback、恢复 |
| 提炼层 | `DistillationProfileService` / `DistillationPreviewService` | 让用户控制“什么值得记、怎么提炼” |

## 8. 预期交付结果

如果 V2.8 完成，系统应达到以下状态：

- 同一 repo / 项目 / 工作集不会被频繁误判成新项目
- 新旧内容之间能明确分成重复、近重复、替代、冲突，而不是全走 append
- 用户能在 CLI / HTTP / MCP 看到待处理 proposal，并批准或拒绝
- 关键记忆有版本时间线，支持查看 diff、rollback、restore
- 用户能配置全局和项目级 distillation profile
- recall 默认继续保守，不会把待审核和冲突项静默混回上下文

## 9. 推荐推进顺序

1. 先完成项目身份与内容关系判定
2. 再完成 proposal / review queue
3. 再完成 version / timeline / rollback
4. 最后补 distillation profile 和 preview

这个顺序可以保证：

- 先把“该不该动”判断清楚
- 再把“怎么给用户看、怎么批准”做清楚
- 最后再做“如何更聪明地提炼”

## 10. 当前实现状态

V2.8 已完成 tasklist 内全部交付项，并通过验收脚本：

```bash
./docs/scripts/v2_8-acceptance.sh
```

发布前门禁入口：

```bash
V2_8_GATE_STRICT=1 ./docs/scripts/v2_8-production-gate.sh
```

最近一次验收报告：

- [`tests/reports/e2e/latest/v2_8-acceptance.txt`](../../tests/reports/e2e/latest/v2_8-acceptance.txt)
- [`tests/reports/e2e/latest/v2_8-surface-parity.txt`](../../tests/reports/e2e/latest/v2_8-surface-parity.txt)
