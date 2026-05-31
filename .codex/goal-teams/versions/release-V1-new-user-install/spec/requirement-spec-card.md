# Requirement Specification Card

## 核心需求

新用户必须能复制公开 README 的安装路径，从 GitHub 下载 release-V1 安装器，安装当前平台二进制，并立即写入一条 memory。

## 业务流程

1. 用户打开 GitHub 或 README。
2. 用户下载 `install.sh`。
3. 用户运行环境检查。
4. 用户安装 release 二进制。
5. 用户执行一条最小 memory 写入命令。
6. 用户搜索该 memory，确认系统可用。

## 边界

- 不以源码仓库本地路径、mock tarball 或开发环境数据库作为完成条件。
- 可以使用本地 markdown-only 配置完成单用户 quickstart。
- PostgreSQL / provider credentials 属于进阶部署，不应阻断第一条本地 memory。
