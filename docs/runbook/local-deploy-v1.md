# V1 本地独立部署 Runbook

## 1. 目标

在一台 macOS 或 Linux 开发机上，以 Docker Compose 方式启动：

- PostgreSQL + pgvector
- memory-app
- memory-worker

并保留：

- PostgreSQL 主存
- Markdown projection
- 本地图片资产目录
- HTTP + MCP 接口

## 2. 前置条件

- Docker / Docker Compose 可用
- Rust 1.85.0 已安装
- 当前仓库已拉取到本地

可先执行：

```bash
./scripts/verify.sh
./scripts/bootstrap.sh
```

## 3. 启动

```bash
cp .env.example .env
./scripts/dev-up.sh
```

说明：

- `scripts/dev-up.sh` 会执行 `docker compose up -d --build pgvector app worker`
- `compose.yaml` 现在会为 Markdown 和 assets 挂载命名卷
- `config/docker.toml` 已开启 `enable_mcp = true`

## 4. 检查

```bash
curl http://127.0.0.1:8080/healthz
curl http://127.0.0.1:8080/readyz
curl http://127.0.0.1:8080/api/v1/meta
curl http://127.0.0.1:8080/mcp/tools
```

## 5. 数据位置

本地 compose 使用三个 volume：

- `pgvector-data`
- `memory-docs`
- `memory-assets`

这保证了容器重建后，PG 数据、Markdown projection 和图片资产不会因为镜像重建而直接丢失。

## 6. 停止

```bash
./scripts/dev-down.sh
```

当前 `dev-down.sh` 只做 stop，不会清空 volume。

## 7. 常见问题

### 7.1 `/mcp/tools` 不可用

检查 `config/docker.toml` 中是否启用了：

```toml
[features]
enable_mcp = true
```

### 7.2 图片写入成功但 vision 信息为空

V1 的 vision 路径不依赖外部 SDK，但仍依赖：

- 正确的 `media_type`
- 有效或至少可识别的图片输入
- 模型 registry 中存在 `vision` route

### 7.3 worker 看起来没有真正消费任务

这是当前 V1 预期行为。worker 在 V1 先承担部署拓扑占位、配置校验和后台心跳角色，真实任务消费会在后续版本继续补强。
