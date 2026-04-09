# V1 中文验收语料

## 目标

这份语料用于把 V1 的“中文优先”从零散样例升级为可维护的系统化回归入口。

## 语料主题

| 场景 | 中文语料 | 预期 |
| --- | --- | --- |
| 文本记忆写入 | `发布前必须通过回归与验收。` | 可被 remember 写入并在中文 query 下检索命中 |
| 中文检索 | `回归 验收`、`发布前 验收` | 能命中对应 memory |
| 中文知识图谱 | ``项目 `服务网关` 依赖 `PostgreSQL`，`服务网关` 记录 `发布手册`。`` | 产生实体 `服务网关` / `PostgreSQL` / `发布手册`，并识别 `DependsOn` / `Documents` 关系 |
| 中文图片语义 | `检测到一张 image/png 图片` | 图片 caption 返回中文优先描述 |
| 中文 failover 提示 | `Gemini Vision LLM 不可用，已经切换到Claude Vision` | 图片写入命中 fallback 时返回中文提示文案 |

## 自动化入口

| 层 | 覆盖入口 |
| --- | --- |
| Kernel | `crates/memory-kernel/tests/kernel_flow_tests.rs` |
| HTTP | `crates/memory-http/tests/http_api_tests.rs` |
| MCP | `crates/memory-mcp/tests/mcp_tools_tests.rs` |
| CLI | `crates/memory-cli/tests/cli_e2e.rs` |
| 统一验收脚本 | `./scripts/v1-acceptance.sh` |

## 维护规则

- 新增中文能力时，优先在现有四条主验收链路中补用例，而不是只写文档样例。
- 中文 query 尽量使用真实工作场景短语，不使用纯造词样本。
- 中文关系样例优先覆盖 `依赖`、`使用`、`引用`、`记录` 这类 V1 高频语义。
