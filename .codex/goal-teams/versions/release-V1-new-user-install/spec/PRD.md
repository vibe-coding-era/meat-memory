# PRD: release-V1 新用户安装与第一条 memory

## 用户目标

作为新用户，我希望不用克隆源码仓库，也不用先理解开发环境，就能安装 Meat Memory 并写入第一条 memory。

## 验收标准

| ID | 标准 |
|---|---|
| PRD-001 | 安装器可从公开 GitHub `release-V1` ref 下载 |
| PRD-002 | 当前平台 release asset 和 `.sha256` 可下载 |
| PRD-003 | 安装器能在空 HOME 下完成安装 |
| PRD-004 | 安装后命令可写入一条 memory |
| PRD-005 | 搜索能返回刚写入的 memory |

## 非目标

- 不要求新用户在第一条 memory 前配置 SaaS connector、OCR/ASR/video provider 或生产数据库。
- 不把源码开发脚本作为 release 安装入口。
