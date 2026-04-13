# Docker Infra

本目录保存 Docker 相关的初始化资产，以及本地 `docker compose` 依赖的初始化文件。

## 当前内容

- `postgres/init/001-init.sql`：本地 PostgreSQL 容器启动时执行的初始化脚本

## 当前镜像约定

当前仓库统一使用：

- `meat-memory-app:<tag>`：HTTP / MCP / Browser Console 主服务镜像
- `meat-memory-worker:<tag>`：后台 worker 镜像

本地 compose 默认使用：

- `meat-memory-app:local`
- `meat-memory-worker:local`

对应构建入口：

```bash
docker build --target app-runtime -t meat-memory-app:local .
docker build --target worker-runtime -t meat-memory-worker:local .
```

## 适用场景

- 查看本地 compose 环境里数据库是如何初始化的
- 补充新的开发库初始化逻辑

如果是日常启动环境，直接使用仓库根的 `compose.yaml` 和 `scripts/dev-up.sh` 即可。
