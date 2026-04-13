# 文档统一管理

本目录作为技术文档中的“文档统一管理”入口，用于定义文档分类、目录职责和维护规则，避免后续文档继续分散生长。

## 怎么使用

新增文档前，先回答两个问题：

1. 这份文档主要是在讲架构、测试，还是文档治理本身？
2. 它是“对外使用说明”，还是“内部执行上下文”，还是“静态分析/汇总报告”？

按这个判断后，再决定放到哪个目录。

## 适合什么时候看

- 想新增一篇文档，但不知道应该放哪里
- 想给现有文档重新分类
- 想统一团队的文档维护规则

## 三大主线

- [`../architecture-design/README.md`](../architecture-design/README.md)：架构设计
- [`../testing/README.md`](../testing/README.md)：测试方案
- [`README.md`](README.md)：文档统一管理

## 目录职责

| 目录 | 角色 |
| --- | --- |
| `docs/architecture-design/` | 架构设计总入口 |
| `docs/testing/` | 测试方案总入口 |
| `docs/document-management/` | 文档治理规则与编目入口 |
| `docs/api/` | 协议与命令文档 |
| `docs/scripts/` | 和技术文档一起管理的项目脚本 |
| `docs/runbook/` | 部署、运行、排障和验收操作说明 |
| `docs/reports/` | 分析型、汇总型报告 |
| `docs/tasks/` | 内部任务与执行上下文 |
| `tests/` | 测试说明、验收语料和运行产物说明 |

## 文档放置规则

### 放到“架构设计”

- 系统分层、模块职责、对象模型、版本架构方案
- 接口设计、接入形态、部署拓扑

### 放到“测试方案”

- 测试分层说明
- 验收标准和语料
- 报告入口、测试覆盖率分析

### 放到“文档统一管理”

- 文档目录说明
- 编目规则、阅读顺序、维护约定
- 文档迁移或归档策略

## 维护原则

1. 所有新增技术文档，先判断归属到哪一条主线，再决定具体目录。
2. 涉及对外使用方式的文档，仍要同步更新根 [`../README.md`](../README.md) 和 [`../runbook/usage-guide.md`](../runbook/usage-guide.md)。
3. 涉及测试入口的文档，除了更新 `docs/testing/`，还要更新 `tests/README.md` 或对应子目录 README。
4. 涉及历史方案但仍需保留参考价值的文档，不删除，保留在原路径并通过分类入口编目。

## 当前编目结论

- `meat-memory-scheme-v*.md`、`system-design.md`、`product-feature-structure.md` 归入“架构设计”
- `tests/*`、`tests/reports/*`、`docs/reports/*`、`runbook/v1-acceptance.md` 归入“测试方案”
- `docs/README.md`、`docs/tasks/README.md`、本页以及各目录导航页归入“文档统一管理”
- `docs/scripts/README.md` 与脚本目录管理规则也归入“文档统一管理”

## 例子

### 例子 1：新增架构文档

如果你要新增“对象模型设计”或“模块分层说明”，优先放到：

- `docs/architecture-design/`
- 或保留在原专题目录后，再通过 `docs/architecture-design/README.md` 编目

### 例子 2：新增测试相关文档

如果你要新增“验收标准”“测试报告说明”“覆盖率总结”，优先放到：

- `docs/testing/`
- `docs/reports/`
- `tests/` 或 `tests/reports/`

### 例子 3：新增目录说明或治理规则

如果你要新增“目录职责”“命名规范”“文档迁移说明”，优先放到：

- `docs/document-management/`
- 或对应目录自己的 `README.md`
