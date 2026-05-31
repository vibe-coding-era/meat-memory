# release-V1-install-flow Decisions

| 时间 | 决策 | 原因 | 影响 |
| --- | --- | --- | --- |
| 2026-05-31 | 使用 `release-V1-install-flow` 作为版本目录 | 目标是 release-V1 安装流程验收，和先前 `release-v1-review` 区分 | 新运行文档独立存放 |
| 2026-05-31 | 不强制安装系统依赖 | 安装器可提示并在 `--install-deps` 时尝试；测试不应擅自改用户系统 | 依赖补齐以 dry-run / check 为主 |
| 2026-05-31 | GitHub Release 404 视为 external blocker，不直接当作脚本 bug | 当前 release assets 未发布，脚本无法下载是合理失败 | 通过本地模拟 asset 测试脚本核心逻辑 |
| 2026-05-31 | 本地未提交 release preflight 变更不纳入本次安装流完成条件 | 已推送分支 HEAD `9644a11` 不包含该 preflight job；该变更依赖未提交脚本和外部门禁配置 | 如需启用 release preflight，应另开任务补齐脚本、waiver/secrets 和验证证据 |
| 2026-05-31 | 最终状态为 `pass-local / blocked-external-release-assets` | 本地 mock release 安装和独立 QA/Reviewer 均通过；真实 GitHub Release tag/assets 尚不存在 | 可交付安装脚本修复，但真实远程安装需正式 release assets |
