# release-V1 新用户安装验收决策

| 时间 | 决策 | 原因 | 影响 |
|---|---|---|---|
| 2026-05-31T20:14:21+0800 | 创建并推送 `release-V1` tag | 新用户 raw installer 与 GitHub Release assets 都依赖该 ref；缺失时公开安装必然 404 | 触发 GitHub Actions Release workflow |
| 2026-05-31T20:16:29+0800 | 验收口径改为真实远程安装和真实 memory 写入 | 用户明确要求不能假定测试、不能用 mock | mock 只保留为辅助，不作为完成证据 |
| 2026-05-31T20:24:00+0800 | 修复安装后默认配置为 markdown-only，并新增 `--verify-write` | 默认开发配置依赖 PostgreSQL，会阻塞新用户第一条 memory | 需要提交后重新指向 `release-V1` tag 并重新发布 assets |
