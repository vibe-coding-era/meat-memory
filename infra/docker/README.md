# Docker Infra

本目录保存 Docker 相关的初始化资产。

## 当前内容

- `postgres/init/001-init.sql`：本地 PostgreSQL 容器启动时执行的初始化脚本

## 适用场景

- 查看本地 compose 环境里数据库是如何初始化的
- 补充新的开发库初始化逻辑

如果是日常启动环境，直接使用仓库根的 `compose.yaml` 和 `scripts/dev-up.sh` 即可。
