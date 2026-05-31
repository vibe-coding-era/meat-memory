# release-V1 新用户安装验收 Tasklist

| ID | Owner | 状态 | 任务 | 完成标准 | 验证 |
|---|---|---|---|---|---|
| GT-NUI-001 | 发布-公开入口资产核验 | in-progress | 核验 `release-V1` tag / raw installer / Release assets | 公开 URL 返回 200 且资产完整 | `curl -I`, GitHub API, `git ls-remote` |
| GT-NUI-002 | 测试-新用户安装复现 | in-progress | 使用空 HOME 从 GitHub 下载并安装 | `install.sh --check` 与 `install.sh --skip-tui` 成功 | 临时 HOME 命令日志 |
| GT-NUI-003 | 测试-真实写入记忆验收 | in-progress | 安装后写入一条 memory | `memory-cli remember --json` 成功且 `search --json` 命中 | 安装后的 release 二进制 |
| GT-NUI-004 | 修复-安装链路缺陷处理 | in-progress | 修复真实新用户链路发现的 bug | 失败点消失，说明可复制 | 已完成 mock release `--verify-write` 回归；待真实 Release assets |
| GT-NUI-005 | 审计-完成门禁 | in-progress | 独立审计未尽项 | 无 P0/P1 未完成项或阻塞明确 | 完成审计报告 |
