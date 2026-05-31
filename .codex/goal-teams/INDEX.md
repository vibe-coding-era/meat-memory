# Goal Teams 索引

更新时间：2026-05-31T17:24:12+0800

本目录记录 Goal Teams 运行过程、版本化交付状态和独立审计结果。历史根目录文件保留为旧运行记录；新运行产物统一写入版本目录。

| 版本 | 状态 | 版本目录 | 说明 |
|---|---|---|---|
| `release-v0.1.0-preflight` | publish-readiness-local-gates-passed / not-release-ready-external-blockers | `versions/release-v0.1.0-preflight/` | V0.1.0 最后发布前测试验证、Bug 修复闭环、发布入口完善、正式 waiver 机制和未尽事宜报告 |
| `release-v1-review` | closed-local / external-gated | `versions/release-v1-review/` | V1 release 复核剩余本地工作已完成；strict release 仍外部/lead-gated |
| `release-V1-install-flow` | pass-local / blocked-external-release-assets | `versions/release-V1-install-flow/` | release-V1 部署安装流程本地全链路通过；真实远程安装等待 `release-V1` tag 与 GitHub Release assets |
| `release-V1-new-user-install` | in-progress-real-user-e2e | `versions/release-V1-new-user-install/` | 以新用户视角从 GitHub 下载、安装，并真实写入一条 memory 的并发 Goal Teams 验收 |
| `legacy-root-v3.1` | indexed-only | root files | 旧根目录 Goal Teams 文件，暂不迁移 |

## 约定

- 过程和结果 Markdown 文档必须进入 `versions/<version>/`。
- 根目录旧文件不迁移、不重写，避免破坏历史 Agent Teams / Goal Teams 记录。
- 外部凭据、公开发布、生产环境和 release lead 决策类事项只记录阻塞条件，不伪造完成。
