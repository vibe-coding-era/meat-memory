# V1 云端独立部署 Runbook

V1 的云部署目标是“单节点、单实例可用”，不是一开始就做成复杂集群。

## 1. 交付物

当前仓库已经提供：

- 运行镜像：`Dockerfile`
- 云配置样例：`config/cloud.example.toml`
- Helm Chart：`infra/helm/meat-memory`

## 2. 构建镜像

```bash
docker build -t meat-memory:v1 .
```

CI 也会执行一次 `docker build`，用来提前发现镜像回归。

## 3. 直接用 Docker 运行

最小方式：

```bash
docker run --rm -p 8080:8080 \
  -v "$(pwd)/config:/app/config:ro" \
  -v "$(pwd)/runtime-data:/data" \
  -e MEAT_MEMORY_CONFIG=/app/config/cloud.example.toml \
  meat-memory:v1
```

说明：

- `cloud.example.toml` 里默认使用 `/data/markdown` 和 `/data/assets`
- 需要你把 `database_url` 改成自己的云 PostgreSQL 地址
- 如果需要接云模型，再补相应 API Key 环境变量

## 4. Helm 部署

```bash
helm lint infra/helm/meat-memory
```

```bash
helm upgrade --install meat-memory infra/helm/meat-memory \
  --set image.repository=meat-memory \
  --set image.tag=v1 \
  --set postgres.databaseUrl='postgres://postgres:postgres@postgresql:5432/meat_memory'
```

如需模型 API Key：

```bash
helm upgrade --install meat-memory infra/helm/meat-memory \
  --set image.repository=meat-memory \
  --set image.tag=v1 \
  --set postgres.databaseUrl='postgres://postgres:postgres@postgresql:5432/meat_memory' \
  --set secrets.geminiApiKey='xxx' \
  --set secrets.openaiApiKey='xxx'
```

## 5. Helm 默认策略

- app 默认 1 副本
- worker 默认关闭
- 默认启用持久卷
- 默认启用 HTTP + MCP

这样做是为了让 V1 更贴合“单实例独立部署”的边界。

## 6. 为什么 worker 默认关闭

V1 的云 chart 默认使用单个 PVC，适合：

- 单 app 实例
- 单节点或共享存储明确的环境

如果 worker 与 app 共享 Markdown / assets 路径，通常需要确认底层卷支持共享访问模式。为了避免误导，chart 默认先关闭 worker，按需开启：

```bash
--set worker.enabled=true
```

## 7. 发布后检查

```bash
curl http://<service>/healthz
curl http://<service>/readyz
curl http://<service>/api/v1/meta
curl http://<service>/mcp/tools
```
