# Acceptance: release-V1 安装流程

## 状态

本地安装流程通过；独立 QA 和 Reviewer 最终确认无 P0/P1；Completion Auditor 的记录闭环问题已处理。真实远程安装仍需要正式推送 `release-V1` tag 并等待 GitHub Actions 产出 release assets。

## 预期结论类型

- `pass-local`: 本地模拟 release asset 安装和配置入口通过。
- `blocked-external-release-assets`: GitHub Release `release-V1` 未发布或 assets 缺失。
- `fail-bug`: 脚本逻辑 bug，需要修复并复测。
- `env-blocked`: 本机缺少依赖、目录不可写、数据库不可达或无交互终端导致无法继续。

## 失败归因矩阵

| 失败现象 | 归因 | 是否阻塞本地脚本发布 |
| --- | --- | --- |
| `release-V1` release/tag/asset/raw install URL 404 | `blocked-external-release-assets` | 否，阻塞真实远程安装 |
| 四个平台 tarball 或 `.sha256` 缺失 | `blocked-external-release-assets` | 否，阻塞真实远程安装 |
| checksum 与 tarball 不匹配 | `blocked-external-release-assets` | 否，阻塞真实远程安装 |
| `bash -n` 失败、参数解析错误、默认擅自安装系统包 | `fail-bug` | 是 |
| 安装完成后退出码非 0 | `fail-bug` | 是 |
| 无 TTY 时默认 TUI 阻塞，或 `dev-up.sh` 触发 TUI | `fail-bug` | 是 |
| 缺 `curl/wget/tar/awk/install/checksum` 且用户不允许补齐 | `env-blocked` | 否 |
| 安装目录/配置目录不可写 | `env-blocked` | 否 |
| `memory-cli config check` warning | `env-blocked` 或待配置 | 否，安装器应继续 TUI 引导 |

## 证据

- 主线程本地模拟 release asset 首次安装：安装动作完成，但退出阶段报 `tmp_dir: unbound variable`，判定为脚本 bug。
- 主线程修复后复测：`install.sh --check --skip-tui` 通过；本地模拟 release asset 安装通过；mock `memory-cli --help`、`memory-cli config check`、`memory-cli tui init --help` 均通过。
- 主线程远程路径复测：GitHub Release asset 404 时退出码为 1，并明确提示检查 `release-V1 release assets` 或 `MEAT_MEMORY_RELEASE_BASE_URL`。
- 主线程 `dev-up.sh` 复测：`MEAT_MEMORY_SKIP_DB_CHECK=1` 时可启动并停止 mock `memory-app` / `memory-worker`；DB `127.0.0.1:1` 不可达时提前失败并提示启动数据库、修改 URL 或设置 skip。
- 主线程 workflow 检查：release workflow 已增加 `release-*` tag 触发，`release-V1` tag 可触发构建；当前 GitHub Release assets 仍需正式 tag push / Actions 成功后产生。
- 主线程外部环境覆盖复测：`.env` 中 `MEAT_MEMORY_SKIP_DB_CHECK=0`，命令行 `MEAT_MEMORY_SKIP_DB_CHECK=1 ./dev-up.sh` 优先生效，skip-db 路径通过。
- QA 最终结论：`completed_pass`，无 P0/P1。
- Reviewer 最终结论：`completed-readonly-no-p0-p1`。
- GitHub 推送：`codex/release-v1-deploy-installer` 已更新到 `9644a11`。
- Completion Auditor 结论：本地安装流程 `pass-local`；真实远程安装 `blocked-external-release-assets`；本地未提交 release preflight 变更需另行闭环，未包含在已推送分支 HEAD。

## 最终结论

| 范围 | 状态 | 说明 |
| --- | --- | --- |
| 本地安装脚本 | `pass-local` | mock release asset 安装、checksum、CLI/TUI 入口、DB preflight、env override 均通过 |
| 已推送 GitHub 分支 | `pushed` | `codex/release-v1-deploy-installer` -> `9644a11` |
| 真实远程安装 | `blocked-external-release-assets` | 远端尚无 `release-V1` tag / GitHub Release assets |
| 本地未提交 release preflight 变更 | `out-of-scope-follow-up` | 不在 `9644a11` 内；若要启用需单独补齐脚本、waiver/secrets 和验证 |
