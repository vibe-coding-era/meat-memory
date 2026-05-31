# release-V1 新用户安装验收 Tasklist

| ID | Owner | 状态 | 任务 | 完成标准 | 验证 |
|---|---|---|---|---|---|
| GT-NUI-001 | 发布-公开入口资产核验 | done | 核验 `release-V1` tag / raw installer / Release assets | 公开 URL 返回 200 且资产完整 | `release-V1^{}` 指向 `f5ed092`；raw installer 200；8/8 Release assets 200 |
| GT-NUI-002 | 测试-新用户安装复现 | done | 使用空 HOME 从 GitHub 下载并安装 | `install.sh --check` 与 `install.sh --skip-tui --verify-write` 成功 | 最终复验临时根目录：`/var/folders/_q/dvmy4f65797c3sy0wbtnb26w0000gn/T//meat-memory-final-real-user.HNS18L` |
| GT-NUI-003 | 测试-真实写入记忆验收 | done | 安装后写入一条 memory | 安装后的 release 二进制完成 `remember/search` | 安装器输出 `memory write verification passed`，并生成 `MEMORY.md` |
| GT-NUI-004 | 修复-安装链路缺陷处理 | done | 修复真实新用户链路发现的 bug | 默认 markdown-only 配置生成、`--verify-write`、README 可复制命令、逐平台 asset 发布、darwin-amd64 runner 阻塞修复均已完成 | 本地 mock 回归 + 真实远程安装 + 全平台资产核验均通过 |
| GT-NUI-005 | 审计-完成门禁 | done | 独立审计未尽项 | 无 P0/P1 未完成项或阻塞明确 | completion auditor 终审结论：可发布，无 P0/P1 |
