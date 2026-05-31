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

## 2026-05-31T20:28:17+0800

| Member | Claimed Tasks | Status | Current Step | Evidence | Next |
|---|---|---|---|---|---|
| 修复-安装链路缺陷处理 | GT-NUI-004 | done-local | 有界修复完成 | `install.sh` 默认生成用户目录 markdown-only 配置；`--verify-write` 调用安装后的 `memory-cli remember/search`；README/README.en/DEPLOYMENT_PLAN 给出可复制命令；`build-release-artifacts.sh` 生成 markdown-only bundle config | 等发布链路产出真实 GitHub Release assets 后，由安装/验收成员执行远程复验 |
| 测试-新用户安装复现 | GT-NUI-002 | blocked-by-assets | 真实远程安装 | 当前本地安装器访问真实 `release-V1` darwin-arm64 asset 返回 404；临时 HOME 本地 release tarball smoke 已通过 | Release asset 可用后重跑远程安装 |
| 测试-真实写入记忆验收 | GT-NUI-003 | blocked-by-assets | 写入验收 | 本地 release tarball `--skip-tui --verify-write` 已写入并搜索 quickstart memory；真实远程证据仍缺 asset | Release asset 可用后用远程下载二进制重跑 |

### 本轮命令证据

- `bash -n deploy/release-V1/install.sh`
- `bash -n docs/scripts/build-release-artifacts.sh`
- 临时 HOME：`HOME=<tmp> bash deploy/release-V1/install.sh --check`
- 本地 release tarball：`HOME=<tmp> MEAT_MEMORY_RELEASE_BASE_URL=file://<tmp-release> bash deploy/release-V1/install.sh --skip-tui --verify-write`
- 发布包构建 smoke：`bash docs/scripts/build-release-artifacts.sh --target release-smoke --asset-name meat-memory-smoke`
- 真实 asset 检查：`HOME=<tmp> bash deploy/release-V1/install.sh --skip-tui --verify-write`，阻塞于 GitHub Release asset 404

## 2026-05-31T20:39:22+0800

| Member | Claimed Tasks | Status | Current Step | Evidence | Next |
|---|---|---|---|---|---|
| 发布-公开入口资产核验 | GT-NUI-001 | done | 公开入口核验 | `release-V1` tag peeled commit `5cb89644ad4e701745a2fa0cb4c4ad284a1bb9e7`；raw installer 200；`meat-memory-darwin-arm64.tar.gz` HEAD 302/200，`content-length: 14994433` | 完成 |
| 测试-新用户安装复现 | GT-NUI-002 | done | 空 HOME 远程安装 | `--check` 退出码 0；`--skip-tui --verify-write` 退出码 0；安装器下载真实 GitHub Release asset 和 `.sha256` | 完成 |
| 测试-真实写入记忆验收 | GT-NUI-003 | done | 第一条 memory 写入 | 安装器输出 `memory write verification passed`；`memory-cli config check --json` 输出 `ok: true`；生成 `.../markdown/default/scopes/scp_release_v1_quickstart/MEMORY.md` | 完成 |
| 修复-安装链路缺陷处理 | GT-NUI-004 | done | 发布修复 | `2cd497a fix: make release V1 install write a first memory`；`5cb8964 ci: publish release assets as each platform builds` | 完成 |
| 审计-完成门禁 | GT-NUI-005 | in-review | 终审 | 已启动 completion auditor | 等审计结论 |

### 真实新用户验收命令摘要

```bash
curl -fsSL https://raw.githubusercontent.com/vibe-coding-era/meat-memory/release-V1/deploy/release-V1/install.sh -o "$tmp_root/install.sh"
chmod +x "$tmp_root/install.sh"
env -i HOME="$tmp_root/home" PATH="/usr/bin:/bin:/usr/sbin:/sbin:/opt/homebrew/bin:/usr/local/bin" TMPDIR="$tmp_root" MEAT_MEMORY_INSTALL_DIR="$tmp_root/install-bin" MEAT_MEMORY_CONFIG_DIR="$tmp_root/config" "$tmp_root/install.sh" --check
env -i HOME="$tmp_root/home" PATH="/usr/bin:/bin:/usr/sbin:/sbin:/opt/homebrew/bin:/usr/local/bin" TMPDIR="$tmp_root" MEAT_MEMORY_INSTALL_DIR="$tmp_root/install-bin" MEAT_MEMORY_CONFIG_DIR="$tmp_root/config" MEAT_MEMORY_RUN_TUI=0 "$tmp_root/install.sh" --skip-tui --verify-write
```

关键输出：

```text
[meat-memory] memory write verification passed
[meat-memory] markdown root: .../home/.local/share/meat-memory/markdown
```

## 2026-05-31T20:52:19+0800

| Member | Claimed Tasks | Status | Current Step | Evidence | Next |
|---|---|---|---|---|---|
| 发布-公开入口资产核验 | GT-NUI-001 | done | 全平台资产复核 | `release-V1` tag peeled commit `f5ed092568daf7a5fbeb6e2efdf4b05e59a12d6d`；8/8 Release assets fresh check 均为 HTTP 200 | 完成 |
| 修复-安装链路缺陷处理 | GT-NUI-004 | done | darwin-amd64 runner 阻塞修复 | `f5ed092 ci: build darwin amd64 release asset on macos 14` 已推送，`meat-memory-darwin-amd64.tar.gz` 与 `.sha256` 均返回 302/200 | 完成 |
| 审计-完成门禁 | GT-NUI-005 | re-review | P1 闭环复审 | completion auditor 上轮 P1 已有修复证据：darwin-amd64 资产可下载，Linux/arm64 资产也全部 200 | 等最终审计结论 |

### 全平台 Release asset fresh check

```text
200 meat-memory-darwin-arm64.tar.gz
200 meat-memory-darwin-arm64.tar.gz.sha256
200 meat-memory-darwin-amd64.tar.gz
200 meat-memory-darwin-amd64.tar.gz.sha256
200 meat-memory-linux-amd64.tar.gz
200 meat-memory-linux-amd64.tar.gz.sha256
200 meat-memory-linux-arm64.tar.gz
200 meat-memory-linux-arm64.tar.gz.sha256
```

### 最终空 HOME 真实远程复验

- 临时根目录：`/var/folders/_q/dvmy4f65797c3sy0wbtnb26w0000gn/T//meat-memory-final-real-user.HNS18L`
- installer 来源：`https://raw.githubusercontent.com/vibe-coding-era/meat-memory/release-V1/deploy/release-V1/install.sh`
- installer SHA256：`a3672d39162469c89c66e9c4256b2335a335d051cc513b8ef4b8e60334132bec`
- `env -i HOME=<tmp>/home ... install.sh --check`：退出码 0
- `env -i HOME=<tmp>/home ... MEAT_MEMORY_RUN_TUI=0 install.sh --skip-tui --verify-write`：退出码 0
- `MEAT_MEMORY_CONFIG=<tmp>/config/app.toml <tmp>/install-bin/memory-cli config check --json`：`ok: true`，`warnings: []`
- 写入文件：`<tmp>/home/.local/share/meat-memory/markdown/default/scopes/scp_release_v1_quickstart/MEMORY.md`

### 独立终审

- 审计成员：`审计-release-V1 新用户安装完成门禁`
- 结论：可发布
- P0/P1 findings：无 P0，无 P1
- 证据：`release-V1` tag 与远端一致；raw installer SHA256 独立复核；8 个公开资产均返回 200；真实新用户安装产物存在；安装后 `memory-cli search` 返回 `memory_count: 1`
- 剩余风险：跨平台方面，本轮确认的是 8 个资产公开可下载，不等同于在 Linux 与 Intel macOS 上分别完成端到端安装运行
