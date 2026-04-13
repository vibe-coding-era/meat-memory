# V1 云端独立部署 Runbook

V1 的云部署目标是“单节点、单实例可用”，不是一开始就做成复杂集群。

## 1. 交付物

当前仓库已经提供：

- 运行镜像：`Dockerfile` 的 `app-runtime` / `worker-runtime` targets
- 云配置样例：`config/app.toml`
- Helm Chart：`infra/helm/meat-memory`

## 2. 构建镜像

```bash
docker build --target app-runtime -t meat-memory-app:v1 .
docker build --target worker-runtime -t meat-memory-worker:v1 .
```

建议镜像命名：

- `meat-memory-app:<tag>`
- `meat-memory-worker:<tag>`

如果使用 GitHub Actions release 流水线，推荐直接消费：

- `ghcr.io/<owner>/meat-memory-app:<tag>`
- `ghcr.io/<owner>/meat-memory-worker:<tag>`

其中 `<tag>` 与 Git tag 或 release 版本保持一致，例如 `v0.4.0`。

## 3. 直接用 Docker 运行

最小方式：

```bash
docker run --rm -p 8080:8080 \
  -v "$(pwd)/config:/app/config:ro" \
  -v "$(pwd)/runtime-data:/data" \
  -e MEAT_MEMORY_CONFIG=/app/config/app.toml \
  -e MEAT_MEMORY_SERVER_BIND=0.0.0.0:8080 \
  -e MEAT_MEMORY_DATABASE_URL='postgres://postgres:postgres@postgresql:5432/meat_memory' \
  -e MEAT_MEMORY_MARKDOWN_ROOT=/data/markdown \
  -e MEAT_MEMORY_ASSETS_ROOT=/data/assets \
  -e MEAT_MEMORY_ENABLE_MCP=true \
  meat-memory-app:v1
```

说明：

- 云端也读取统一的 `config/app.toml`
- 建议通过 `MEAT_MEMORY_DATABASE_URL`、`MEAT_MEMORY_MARKDOWN_ROOT`、`MEAT_MEMORY_ASSETS_ROOT` 覆盖云 PostgreSQL 地址和持久卷目录
- 如果需要接云模型，再补相应 API Key 环境变量

如果需要单独运行 worker：

```bash
docker run --rm \
  -v "$(pwd)/config:/app/config:ro" \
  -v "$(pwd)/runtime-data:/data" \
  -e MEAT_MEMORY_CONFIG=/app/config/app.toml \
  -e MEAT_MEMORY_DATABASE_URL='postgres://postgres:postgres@postgresql:5432/meat_memory' \
  -e MEAT_MEMORY_MARKDOWN_ROOT=/data/markdown \
  -e MEAT_MEMORY_ASSETS_ROOT=/data/assets \
  meat-memory-worker:v1
```

## 4. Helm 部署

```bash
helm lint infra/helm/meat-memory
```

```bash
helm upgrade --install meat-memory infra/helm/meat-memory \
  --set image.repository=ghcr.io/<owner>/meat-memory-app \
  --set image.tag=v1 \
  --set postgres.databaseUrl='postgres://postgres:postgres@postgresql:5432/meat_memory'
```

如需模型 API Key：

```bash
helm upgrade --install meat-memory infra/helm/meat-memory \
  --set image.repository=ghcr.io/<owner>/meat-memory-app \
  --set image.tag=v1 \
  --set postgres.databaseUrl='postgres://postgres:postgres@postgresql:5432/meat_memory' \
  --set secrets.geminiApiKey='xxx' \
  --set secrets.openaiApiKey='xxx'
```

如果你已经在集群里准备好了现成 Secret 和 PVC，推荐覆盖：

```bash
helm upgrade --install meat-memory infra/helm/meat-memory \
  --set image.repository=ghcr.io/<owner>/meat-memory-app \
  --set workerImage.repository=ghcr.io/<owner>/meat-memory-worker \
  --set image.tag=v1 \
  --set worker.enabled=true \
  --set persistence.existingClaim=meat-memory-data \
  --set secrets.existingSecretName=meat-memory-secrets
```

## 5. Helm 默认策略

- app 默认 1 副本
- worker 默认关闭
- 默认启用持久卷
- 默认启用 HTTP + MCP
- 支持单独覆盖 `workerImage.repository`
- 支持复用已有 `Secret` 和 `PVC`

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
