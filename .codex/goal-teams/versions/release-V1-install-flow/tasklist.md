# release-V1-install-flow Tasklist

| Task ID | Owner | Status | Claimed By | Locked Scope | Deliverable | Done Criteria | Verification |
| --- | --- | --- | --- | --- | --- | --- | --- |
| GT-001 | 需求分析-release-V1 安装验收梳理 | done | Researcher A the 3rd | `.codex/goal-teams/versions/release-V1-install-flow/spec/` | 安装验收需求卡和轻量 PRD | 安装目标、边界、bug vs external blocker 清晰 | 审计-release-V1 安装完成审计 |
| GT-002 | 测试-release-V1 安装全流程验收 | done | QA A the 3rd | 临时目录、`deploy/release-V1/` 只读优先 | 安装测试证据 | 环境检查、本地模拟 asset、安装后命令、TUI 入口均有证据；最终 QA 无 P0/P1 | 评审-release-V1 安装脚本审查 |
| GT-003 | 评审-release-V1 安装脚本审查 | done | Reviewer B the 4th | `deploy/release-V1/install.sh`, `deploy/release-V1/dev-up.sh`, docs | 风险审查和修复建议 | 最终复审无 P0/P1 | 审计-release-V1 安装完成审计 |
| GT-004 | 修复-release-V1 安装阻塞问题 | done | Goal Lead + Implementer C the 4th | `deploy/release-V1/`, `.github/workflows/release.yml` tag trigger | 最小修复 | trap、download error、remote install docs、DB preflight、env override、release tag trigger 均已修复并复测 | 测试-release-V1 安装全流程验收 |
| GT-005 | 审计-release-V1 安装完成审计 | done | Reviewer C the 4th | Goal Teams 记录和测试证据 | 完成审计 | 审计发现记录状态缺口和本地 preflight 未提交风险；Lead 已关闭记录，并将 preflight 风险标为本次已推送分支外的另行闭环事项 | Goal Lead |

## 当前状态

- 2026-05-31：用户确认执行，开始 Goal Teams 测试。
- 2026-05-31：本地安装流 `pass-local`；真实远程安装 `blocked-external-release-assets`，需发布 `release-V1` tag/assets。
