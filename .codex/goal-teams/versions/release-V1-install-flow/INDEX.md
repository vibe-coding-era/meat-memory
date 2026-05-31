# release-V1-install-flow Goal Teams 索引

更新时间：2026-05-31

## 目标

验证 `deploy/release-V1/` 的安装流程是否真实可用：环境检测、依赖提示/可选补齐、release 二进制安装、TUI 配置入口、部署目录启动路径、安装结果独立审计。

## 文档

| 文档 | Owner | 状态 | 说明 |
| --- | --- | --- | --- |
| `plan.md` | Goal Lead | ready | 用户确认后的执行计划 |
| `tasklist.md` | Goal Lead | ready | 成员任务、验收标准、验证归属 |
| `progress.md` | Goal Lead | done | 过程进度和证据 |
| `decisions.md` | Goal Lead | ready | 决策、风险和外部 blocker |
| `goal-packet.md` | Goal Lead | ready | 团队级目标包 |
| `spec/requirement-spec-card.md` | 需求分析-release-V1 安装验收梳理 | ready | 安装验收需求卡 |
| `spec/PRD.md` | 需求分析-release-V1 安装验收梳理 | ready | 轻量 PRD |
| `spec/architecture-design.md` | 文档-release-V1 安装流程记录 | ready | 安装流与测试夹具设计 |
| `spec/test-plan.md` | 测试-release-V1 安装全流程验收 | ready | 测试计划 |
| `spec/acceptance.md` | 审计-release-V1 安装完成审计 | done | 最终验收证据 |

## 当前状态

- GitHub 分支：`codex/release-v1-deploy-installer`
- 当前提交：`9644a11`
- 已确认远端分支提交一致。
- 本地安装流程：`pass-local`。
- 已知外部 blocker：GitHub Release tag `release-V1` / release assets 尚未发布，真实远程安装需正式 tag push 并等待 Actions 产物。
- 本地工作区未提交的 release preflight 变更不属于本次已推送安装流；如需启用，应另行提交其脚本、waiver/secrets 配置和验证证据。
