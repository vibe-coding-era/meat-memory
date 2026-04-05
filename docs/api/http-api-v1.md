# V1 HTTP API

## 1. 入口概览

默认基址：

```text
http://127.0.0.1:8080
```

V1 已实现的 HTTP 路由：

| 路由 | 方法 | 用途 |
|---|---|---|
| `/healthz` | `GET` | 进程健康检查 |
| `/readyz` | `GET` | 读写依赖准备度 |
| `/livez` | `GET` | 存活检查 |
| `/metrics` | `GET` | 内存指标快照 |
| `/api/v1/meta` | `GET` | 服务元信息与 feature flag |
| `/api/v1/memories` | `POST` | 写入文本记忆 |
| `/api/v1/images` | `POST` | 写入图片记忆 |
| `/api/v1/context` | `POST` | 搜索上下文，兼容别名 |
| `/api/v1/context/search` | `POST` | 搜索上下文 |

## 2. 写入文本

```bash
curl -X POST http://127.0.0.1:8080/api/v1/memories \
  -H 'content-type: application/json' \
  -d '{
    "scope_id": "scp_demo_http",
    "title": "上线规则",
    "body": "生产变更必须先通过回归测试。",
    "memory_kind": "procedure",
    "visibility": "private",
    "sensitivity": "internal"
  }'
```

成功后返回：

- `artifact_id`
- `memory_id`
- `scope_id`
- `memory_kind`
- `memory_state`
- `evidence_count`
- `wrote_pg`
- `wrote_markdown`

## 3. 写入图片

图片接口要求传 `image_base64`。

macOS 示例：

```bash
IMAGE_B64="$(base64 < ./samples/login.png | tr -d '\n')"

curl -X POST http://127.0.0.1:8080/api/v1/images \
  -H 'content-type: application/json' \
  -d "{
    \"scope_id\": \"scp_demo_http\",
    \"title\": \"登录页截图\",
    \"body\": \"Codex 登录页截图\",
    \"media_type\": \"image/png\",
    \"image_base64\": \"${IMAGE_B64}\"
  }"
```

V1 图片返回会额外包含：

- `asset_id`
- `asset_uri`
- `vision_caption`
- `vision_model_alias`

说明：

- V1 使用“本地最小 vision derivation + model route attribution”。
- 也就是说，当前不会强依赖真实云视觉 SDK 才能完成图片记忆写入。
- 但返回里的 `vision_model_alias` 仍会标注当前命中的视觉路由别名，便于后续真实 SDK 对接。

## 4. 搜索上下文

```bash
curl -X POST http://127.0.0.1:8080/api/v1/context/search \
  -H 'content-type: application/json' \
  -d '{
    "scope_id": "scp_demo_http",
    "query": "上线规则 回归测试",
    "limit": 5
  }'
```

返回中包含：

- `query`
- `scope_id`
- `memory_count`
- `memories[]`

当前检索是 PG 主存 + graph context 的最小闭环，适合 V1 的文本/图片派生文本召回。

## 5. 元信息与健康检查

```bash
curl http://127.0.0.1:8080/api/v1/meta
curl http://127.0.0.1:8080/healthz
curl http://127.0.0.1:8080/readyz
curl http://127.0.0.1:8080/livez
curl http://127.0.0.1:8080/metrics
```

`/api/v1/meta` 会返回：

- 服务名和版本
- 默认 scope
- `pg / markdown / http / mcp` 功能开关

## 6. 错误语义

- `400 Bad Request`：参数非法，例如未知 `memory_kind`
- `403 Forbidden`：命中策略限制，例如受限可见性/敏感级组合
- `500 Internal Server Error`：存储、配置或运行时错误
