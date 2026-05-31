# Acceptance: release-V1 新用户安装与第一条 memory

状态：`in-progress`

## 当前证据

| 项 | 状态 | 证据 |
|---|---|---|
| `release-V1` tag | pass | 已通过 SSH 推送，GitHub API 可解析 `deploy/release-V1/install.sh?ref=release-V1` |
| raw installer | pass-with-cache-note | cachebust URL 返回 200；无 cache URL 可能短时命中旧 404 CDN 缓存 |
| GitHub Release assets | pending | Release workflow `26712316492` 构建中 |
| 新用户安装 | pending | 等 release asset；mock release `--verify-write` 已通过 |
| 写入 memory | pending | mock release 已写入并搜索 quickstart memory；最终仍等远程 release asset |

## 最终结论

待补。
