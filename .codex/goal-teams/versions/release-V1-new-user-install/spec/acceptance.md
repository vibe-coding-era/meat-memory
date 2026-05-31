# Acceptance: release-V1 新用户安装与第一条 memory

状态：`pass`

## 当前证据

| 项 | 状态 | 证据 |
|---|---|---|
| `release-V1` tag | pass | `refs/tags/release-V1^{}` 指向 `f5ed092568daf7a5fbeb6e2efdf4b05e59a12d6d` |
| raw installer | pass | `https://raw.githubusercontent.com/vibe-coding-era/meat-memory/release-V1/deploy/release-V1/install.sh` 返回 200，脚本 SHA256 为 `a3672d39162469c89c66e9c4256b2335a335d051cc513b8ef4b8e60334132bec` |
| GitHub Release assets | pass | `darwin-arm64`、`darwin-amd64`、`linux-amd64`、`linux-arm64` 的 tarball 与 `.sha256` 共 8 个资产 fresh check 均返回 HTTP 200 |
| 新用户安装 | pass | 最终复验临时根目录 `/var/folders/_q/dvmy4f65797c3sy0wbtnb26w0000gn/T//meat-memory-final-real-user.HNS18L`；空 HOME、空安装目录、空配置目录，`--check` 与 `--skip-tui --verify-write` 退出码均为 0 |
| 写入 memory | pass | `--verify-write` 输出 `memory write verification passed`；`memory-cli config check --json` 为 `ok: true` 且 `warnings: []`；生成 `.../markdown/default/scopes/scp_release_v1_quickstart/MEMORY.md` |

## 最终结论

当前平台 macOS arm64 的真实新用户安装与第一条 memory 写入验收通过，全平台 Release assets 可公开下载。completion auditor 独立终审结论：可发布，无 P0/P1。

## 剩余风险

- 本轮端到端安装运行在 macOS arm64 当前平台完成；Linux 与 Intel macOS 已核验资产公开可下载，但未在对应真实机器上分别执行完整端到端安装。
