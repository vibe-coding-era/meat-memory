# Team Goal Packet: release-V1-install-flow

## Goal

验证 `deploy/release-V1/` 的安装流程从环境检查到安装后 TUI 配置是否可用，并在发现真实脚本 bug 时完成最小修复、复测、提交和推送。

## Success Criteria

- `install.sh --check --skip-tui` 通过。
- 本地模拟 GitHub Release asset 安装通过。
- 安装后的 `memory-cli config check` 可运行。
- TUI 配置入口可达，自动化场景可跳过 TUI。
- `dev-up.sh` 不会在后台启动路径中触发交互 TUI。
- 独立 QA、Reviewer、Completion Auditor 给出完成或 blocker 结论。

## Allowed Scope

- `deploy/release-V1/`
- `.codex/goal-teams/versions/release-V1-install-flow/`
- `.codex/goal-teams/INDEX.md`

## Forbidden Scope

- 不修改源码 crates、SDK、workflow 或 release build 脚本，除非发现与安装流程直接相关且用户确认。
- 不提交无关脏改动。
- 不创建正式 GitHub Release 或 tag。

## Tests

- `bash -n deploy/release-V1/install.sh deploy/release-V1/dev-up.sh deploy/release-V1/dev-down.sh`
- `deploy/release-V1/install.sh --check --skip-tui`
- 本地 HTTP/file 方式模拟 release asset，执行 `install.sh --skip-tui`
- 安装后执行 `memory-cli --help`、`memory-cli config check`
- TUI 非交互可达性：`memory-cli tui init --help` 或等价命令
