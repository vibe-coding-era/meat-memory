# V1 验收说明

## 1. 验收目标

V1 需要确认以下闭环：

| 维度 | 验收点 |
|---|---|
| 存储 | PG 主存 + Markdown projection 同步写入 |
| 文本 | remember / search / publish 可用 |
| 图片 | remember-image 可写入、可返回资产 URI 和 vision 元数据 |
| Agent 接入 | HTTP / CLI / MCP 都有可执行入口 |
| 模型抽象 | provider/model/route registry 可命中 mock provider 配置 |
| 部署 | compose 本地栈和 Docker/Helm 云部署骨架可运行 |

## 2. 自动化入口

```bash
./scripts/v1-acceptance.sh
```

或：

```bash
just acceptance
```

该脚本当前会执行：

- `memory-kernel` 集成流
- `memory-http` 集成流
- `memory-mcp` 集成流
- `memory-cli` 端到端流

## 3. 覆盖范围说明

### 3.1 HTTP

- 文本写入
- 搜索
- 图片写入
- vision caption 返回

### 3.2 CLI

- 文本 remember/search
- 图片 remember-image
- mock provider registry 命中 `vision_model_alias`

### 3.3 MCP

- `memory.remember`
- `memory.search`
- `memory.fetch_context`
- `memory.publish`

## 4. V1 对图片的验收口径

V1 不要求真实云视觉 SDK 全接入，验收口径是：

- 能稳定写入图片资产
- 能自动生成中文优先 caption
- 能把派生文本写回可检索 memory
- 能返回命中的视觉模型路由别名

## 5. 手工补充检查

自动化通过后，建议再手动补两步：

1. `docker compose up -d --build` 后检查 `/mcp/tools` 与 `/api/v1/meta`
2. `helm lint infra/helm/meat-memory`，确认 chart 模板可解析
