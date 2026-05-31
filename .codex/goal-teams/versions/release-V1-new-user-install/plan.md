# release-V1 新用户安装验收计划

## 目标

以真实新用户视角完成：

1. 从 GitHub `release-V1` 公开入口下载 `install.sh`。
2. 在空 HOME / 空安装目录 / 空配置目录执行 `--check`。
3. 从 GitHub Release 下载当前平台 release asset 与 `.sha256` 并完成安装。
4. 使用安装后的 `memory-cli` 写入一条 memory。
5. 搜索或读取该 memory，证明写入真实可用。

## 假设

- 验收平台为当前执行环境：macOS / `arm64`，目标资产为 `meat-memory-darwin-arm64.tar.gz`。
- 远程安装不得使用 `MEAT_MEMORY_RELEASE_BASE_URL=file://...` 或本地 `deploy/release-V1/install.sh`。
- 写入 memory 可以使用新用户可复制的环境变量配置，但必须使用安装后的 release 二进制。
- 若默认配置依赖 PostgreSQL 导致写入失败，必须修复安装说明、安装器或发布资产，而不是把失败当作通过。

## 并发成员

| 成员 | 任务 | 状态 |
|---|---|---|
| 发布-公开入口资产核验 | 检查 tag、raw installer、GitHub Release assets | running |
| 测试-新用户安装复现 | 空环境远程下载安装 | running |
| 测试-真实写入记忆验收 | 形成并执行真实 `memory-cli remember` 验收路径 | running |
| 修复-安装链路缺陷处理 | 对发现的安装/文档/发布链路 bug 做最小修复 | pending |
| 审计-完成门禁 | 独立审查是否满足新用户可用 | running |

## 停止条件

- 完成：远程下载安装成功，且安装后的二进制成功写入并检索一条 memory。
- 继续：发现脚本、文档、配置或 release workflow 缺陷。
- 阻塞：无 GitHub 发布权限、GitHub Actions 无法完成、或外部服务不可用且不能由仓库修复。
