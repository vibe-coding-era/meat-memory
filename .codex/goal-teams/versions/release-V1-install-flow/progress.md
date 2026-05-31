# release-V1-install-flow Progress

## Round 0 - 启动

| Member | Claimed Tasks | Status | Current Step | Evidence | Next |
| --- | --- | --- | --- | --- | --- |
| Goal Lead | 全局协调 | done | GitHub 远端确认 | 本地 `877cfdd` 与远端 `877cfdd1...` 一致 | 启动 subagents |
| 需求分析-release-V1 安装验收梳理 | GT-001 | queued-then-done | 等待 subagent | - | 输出需求卡/PRD建议 |
| 测试-release-V1 安装全流程验收 | GT-002 | queued-then-done | 等待 subagent | - | 运行安装测试 |
| 评审-release-V1 安装脚本审查 | GT-003 | queued-then-done | 等待 subagent | - | 审查脚本和风险 |
| 修复-release-V1 安装阻塞问题 | GT-004 | queued-then-done | 等待测试结果 | - | 必要时修复 |
| 审计-release-V1 安装完成审计 | GT-005 | queued-then-done | 等待集成 | - | 最终审计 |

## Round 1 - 主线程本地复现与修复

| Member | Claimed Tasks | Status | Current Step | Evidence | Next |
| --- | --- | --- | --- | --- | --- |
| Goal Lead | GT-004 | fixed-awaiting-review | 本地 mock release asset 安装复现并修复 bug | 首次模拟安装完成后报 `deploy/release-V1/install.sh: line 412: tmp_dir: unbound variable`，退出码 1；修复后同一流程退出码 0 | 等待 QA/Reviewer 独立复核 |

## Bug 记录

| ID | Severity | 现象 | 根因 | 修复 |
| --- | --- | --- | --- | --- |
| BUG-001 | P1 | 安装流程完成后仍返回失败退出码 | `trap 'rm -rf "${tmp_dir}"' EXIT` 在 `main` 返回后访问局部变量，`set -u` 下触发 unbound variable | 新增全局 `TMP_DIR_TO_CLEAN` 和 `cleanup()`，trap 调用函数 |
| BUG-002 | P2 | GitHub Release asset 404 时只显示原始 curl 错误 | 下载失败缺少面向用户的恢复提示 | `download_file` 捕获 curl/wget 失败并提示检查 `release-V1` assets 或 `MEAT_MEMORY_RELEASE_BASE_URL` |
| DOC-001 | P2 | README 要求运行 `memory-app --help` / `memory-worker --help`，可能误触发长驻服务 | app/worker 是服务入口，不一定是 help CLI | 改为 `command -v memory-app` / `command -v memory-worker`，并说明通过进程管理器启动 |
| DOC-002 | P1 | 远程安装示例 `curl | bash --install-deps` 风险过高 | 未审阅远程脚本就可能触发系统包安装和 TUI | 改为下载、审阅、`--check`、`--skip-tui` 的分步命令 |
| BUG-003 | P1 | `dev-up.sh` 在无 PostgreSQL/pgvector 时直接启动服务失败 | 缺少 DB 前置检查 | 启动前解析 `MEAT_MEMORY_DATABASE_URL` 并用 `nc` 做 TCP preflight；`nc` 缺失或 DB 不可达时明确失败；允许 `MEAT_MEMORY_SKIP_DB_CHECK=1` 跳过 |
| BUG-004 | P1 | `release-V1` tag 不触发 release workflow，默认安装找不到 assets | workflow 只监听 `v*` tag | workflow 增加 `release-*` tag 触发，保留 `v*` |
| BUG-005 | P1 | `MEAT_MEMORY_SKIP_DB_CHECK=1 ./dev-up.sh` 被 `.env` 中的默认 `0` 覆盖 | `dev-up.sh` source `.env` 后未恢复外部环境变量优先级 | source 前捕获常用 `MEAT_MEMORY_*`/`RUST_LOG`，source 后恢复外部覆盖 |

## Round 2 - P1/P2 修复后复测

| Member | Claimed Tasks | Status | Current Step | Evidence | Next |
| --- | --- | --- | --- | --- | --- |
| Goal Lead | GT-002/GT-004 | passed-local | 语法、环境检查、mock release 安装、dev-up DB preflight | `bash -n` 通过；`install.sh --check --skip-tui` 通过；mock release 安装通过；`dev-up.sh` skip-db 可启动/停止 mock app-worker；DB 127.0.0.1:1 不可达时提前失败 | 等待 QA/Reviewer 续验 |
| Goal Lead | GT-002 | blocked-external | 默认 GitHub release asset 下载路径 | 远程安装退出码 1，提示 `failed to download ... verify release-V1 release assets or MEAT_MEMORY_RELEASE_BASE_URL` | 需要正式发布 `release-V1` assets |
| Goal Lead | GT-003/GT-004 | fixed-awaiting-final-audit | workflow tag 触发修复 | release workflow 增加 `release-*` tag，保留 `v*` | 需提交推送后由 GitHub Actions 在 tag push 时生效 |
| Goal Lead | GT-002/GT-004 | passed-local | 外部环境变量覆盖 `.env` | `.env` 保持 `MEAT_MEMORY_SKIP_DB_CHECK=0` 时，`MEAT_MEMORY_SKIP_DB_CHECK=1 ./dev-up.sh` 可跳过 DB preflight 并启动/停止 mock app-worker | 等待最终审计 |
| Goal Lead | GT-004 | pushed | 修复提交到 GitHub | 推送 `877cfdd..9644a11` 到 `codex/release-v1-deploy-installer` | 等待最终审计 |

## Round 3 - 独立 subagent 结论

| Member | Claimed Tasks | Status | Current Step | Evidence | Next |
| --- | --- | --- | --- | --- | --- |
| 需求分析-release-V1 安装验收梳理 | GT-001 | done | 需求/边界审查 | 区分脚本 bug、环境问题、release asset blocker；建议补 acceptance matrix | 完成 |
| 测试-release-V1 安装全流程验收 | GT-002 | done | 最终 QA | `completed_pass`；无 P0/P1；验证外部 env 覆盖 `.env` | 完成 |
| 评审-release-V1 安装脚本审查 | GT-003 | done | 最终复审 | `completed-readonly-no-p0-p1`；确认 release-* trigger、DB preflight、远程安装示例、tarball 结构、post-install 验证均已解决 | 完成 |
| 修复-release-V1 安装阻塞问题 | GT-004 | done | 修复复核 | Implementer 确认 trap bug 修复有效；未再编辑文件 | 完成 |

## Round 4 - Completion Audit

| Member | Claimed Tasks | Status | Current Step | Evidence | Next |
| --- | --- | --- | --- | --- | --- |
| 审计-release-V1 安装完成审计 | GT-005 | done | 最终审计 | 安装脚本层面无新 P0/P1；指出记录状态缺口、真实 release assets 缺失、本地未提交 preflight 变更风险 | Lead 已关闭记录；preflight 风险标为另行闭环事项 |
| Goal Lead | GT-005 | done | 审计响应 | 远端 `codex/release-v1-deploy-installer` 为 `9644a11`；该 HEAD 的 release workflow 只包含 `release-*` tag 触发修复，不含本地未提交 preflight job | 完成 |

## 独立验证

| Artifact | Author | Validator | Method | Status | Evidence |
| --- | --- | --- | --- | --- | --- |
| `deploy/release-V1/install.sh` | Goal Lead | 评审-release-V1 安装脚本审查 | 代码审查 + 安装测试 | done | Reviewer 最终复审无 P0/P1；QA mock 安装通过 |
| 本地模拟 release 安装结果 | 测试-release-V1 安装全流程验收 | 审计-release-V1 安装完成审计 | 独立复核 | done | QA `completed_pass`；Completion Auditor 确认本地安装流程 `pass-local` |
| `.github/workflows/release.yml` tag trigger | Goal Lead | 评审-release-V1 安装脚本审查 | 工作流触发规则审查 | done | HEAD `9644a11` 支持 `release-*` tag；本地 preflight 未提交变更另行处理 |
