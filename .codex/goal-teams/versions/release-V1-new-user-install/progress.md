# release-V1 新用户安装验收进度

## 2026-05-31T20:16:29+0800

| Member | Claimed Tasks | Status | Current Step | Evidence | Next |
|---|---|---|---|---|---|
| 发布-公开入口资产核验 | GT-NUI-001 | running | 公开入口核验 | 初始 `release-V1` raw installer 返回 404；SSH 推送 tag 后 GitHub API 可读取 installer 内容 | 等待 Release assets |
| 测试-新用户安装复现 | GT-NUI-002 | running | 空环境远程安装 | tag 推送前 raw installer 404；tag 推送后带 cachebust 的 raw installer 返回 200 | 等待 Release asset 后执行安装 |
| 测试-真实写入记忆验收 | GT-NUI-003 | running | 写入路径分析 | CLI 支持 `remember --scope-id --title --body --memory-kind --json`；可用 `MEAT_MEMORY_ENABLE_PG=0` 走 markdown-only | 安装后用 release 二进制执行 |
| 审计-完成门禁 | GT-NUI-005 | running | 初审 | 当前不能以 mock 作为完成 | 等最终证据 |

## 发布动作

- 已创建并通过 SSH 推送 `release-V1` tag，指向 `b8aea0ea7299f8f5bd3a31242ae2eb7dc0a0cd34`。
- GitHub Actions Release run: `26712316492`，当前构建中。

## 2026-05-31T20:24:00+0800

| Member | Claimed Tasks | Status | Current Step | Evidence | Next |
|---|---|---|---|---|---|
| 发布-公开入口资产核验 | GT-NUI-001 | waiting | Release assets 等待 | `darwin-arm64`、`linux-amd64`、`linux-arm64` Actions artifacts 已成功；`darwin-amd64` 仍 queued，Release assets 仍 404 | 等 workflow publish-release |
| 测试-新用户安装复现 | GT-NUI-002 | blocked-by-assets | 真实远程安装 | tag/raw installer 可用；GitHub Release asset URL 仍 404 | 等 assets 后重跑 |
| 测试-真实写入记忆验收 | GT-NUI-003 | in-progress | 修复验证 | 确认 markdown-only 可作为无 PG 首次写入路径 | 等安装后执行 |
| 修复-安装链路缺陷处理 | GT-NUI-004 | implemented-local | 安装器修复 | `install.sh` 新增用户目录 markdown-only 配置生成与 `--verify-write`；mock release 安装实写 `MEMORY.md` 通过 | 提交并重发 `release-V1` tag |
| 审计-完成门禁 | GT-NUI-005 | running | P0 追踪 | P0-1 assets 未发布；P0-2 默认配置卡 PG 已修复待真实验证 | 等最终审计 |

### 本地回归证据

- `bash -n deploy/release-V1/install.sh deploy/release-V1/dev-up.sh deploy/release-V1/dev-down.sh docs/scripts/build-release-artifacts.sh` 通过。
- mock release tarball 安装命令使用 `--skip-tui --verify-write` 通过。
- 生成配置关键字段：
  - `markdown.root = <temp HOME>/.local/share/meat-memory/markdown`
  - `assets.root = <temp HOME>/.local/share/meat-memory/assets`
  - `sync.mode = "local_only"`
  - `enable_pg = false`
  - `enable_markdown = true`
- 写入文件存在：`.../markdown/default/scopes/scp_release_v1_quickstart/MEMORY.md`。
