# 文档总览

`docs/` 现在按三条主线统一管理：`架构设计`、`测试方案`、`文档统一管理`。

## 三大分类入口

- [`architecture-design/README.md`](architecture-design/README.md)：系统架构、版本方案、接口与部署设计
- [`testing/README.md`](testing/README.md)：测试分层、验收入口、测试报告与覆盖率分析
- [`document-management/README.md`](document-management/README.md)：文档分类、目录职责和维护规则
- [`V2.7/README.md`](V2.7/README.md)：三层记忆骨架升级为生命周期系统的版本方案

## 推荐阅读顺序

1. 先看 [`../README.md`](../README.md) 了解项目定位和快速入口
2. 如果要安装或部署，直接看 [`runbook/install.md`](runbook/install.md)
3. 再按目的进入三大分类入口
4. 需要具体操作时，再回到 `runbook/`、`api/`、`tests/` 等细分目录

## 按分类查看

### 架构设计

- [`architecture-design/README.md`](architecture-design/README.md)
- [`architecture-design/install-package-deploy-plan.md`](architecture-design/install-package-deploy-plan.md)
- [`V2.7/README.md`](V2.7/README.md)
- [`architecture/README.md`](architecture/README.md)
- [`api/README.md`](api/README.md)
- [`agent-skills/README.md`](agent-skills/README.md)
- [`scripts/README.md`](scripts/README.md)

### 测试方案

- [`testing/README.md`](testing/README.md)
- [`reports/README.md`](reports/README.md)
- [`runbook/v1-acceptance.md`](runbook/v1-acceptance.md)

### 文档统一管理

- [`document-management/README.md`](document-management/README.md)
- [`tasks/README.md`](tasks/README.md)
- [`runbook/README.md`](runbook/README.md)

## 保留目录导航

- [`runbook/README.md`](runbook/README.md)：部署、启动、验收和排障入口
- [`runbook/install.md`](runbook/install.md)：安装、打包与部署入口
- [`api/README.md`](api/README.md)：CLI、HTTP、MCP 文档入口
- [`V2.7/README.md`](V2.7/README.md)：V2.7 生命周期系统方案入口
- [`scripts/README.md`](scripts/README.md)：文档内统一管理的项目脚本
- [`default/README.md`](default/README.md)：默认 Markdown projection 与示例内容说明
- [`reports/README.md`](reports/README.md)：分析型报告入口
- [`tasks/README.md`](tasks/README.md)：项目任务与执行上下文入口

## 维护约定

- 新增技术文档时，先归类到三大主线之一，再决定具体目录
- 对外使用方式变化时，优先更新根 `README.md` 与 `runbook/usage-guide.md`
- 测试入口或报告变化时，同时更新 `docs/testing/README.md` 与 `tests/` 下对应 README
