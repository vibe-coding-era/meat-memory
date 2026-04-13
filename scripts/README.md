# Scripts Index

本目录放置本地开发、验收、测试报告和运维辅助脚本。

## 环境与启动

- `bootstrap.sh`：初始化本地 Rust 开发环境
- `verify.sh`：检查工具链、Docker、数据库等基础条件
- `dev-db-up.sh` / `dev-db-down.sh`：仅启动或停止开发数据库
- `dev-up.sh` / `dev-down.sh`：启动或停止完整本地栈

## 测试与验收

- `test-required.sh`：仓库必跑测试入口
- `v1-acceptance.sh`：V1 验收
- `v2_1-acceptance.sh`：V2.1 配置 / MCP / TUI / skill 验收
- `v2_4-acceptance.sh`：V2.4 context / docs / source 验收
- `v2-perf.sh`：性能烟测
- `security-report.sh`：安全专项报告
- `write-test-reports.sh`：统一刷新测试报告

## 开发辅助

- `export-agent-skills.sh`：导出 Agent Skill 包
- `install-hooks.sh`：安装 Git hooks

建议先从 [`../docs/runbook/usage-guide.md`](../docs/runbook/usage-guide.md) 进入，再回到这里查具体脚本。
