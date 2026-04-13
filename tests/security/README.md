# Security Tests

本目录用于承载安全测试设计、执行入口和专项说明。

当前重点覆盖：

- `authorization`：敏感接口必须需要合法 key，且不能跨 scope 越权
- `hijack prevention`：Bearer、`x-meat-memory-key` 伪造、匿名访问与提权路径
- `injection boundaries`：SQL、Markdown、路径、header、tool arguments、base64 等输入边界
- `payload limits`：body、query、prompt、image 等资源消耗型输入限制

当前执行入口：

- HTTP / 集成安全回归：`cargo test -p memory-http --test http_api_tests --quiet -- --test-threads=1`
- MCP / 敏感工具回归：`cargo test -p memory-mcp --test mcp_tools_tests --quiet -- --test-threads=1`
- 报告汇总：`./scripts/security-report.sh`

报告目录见 `../reports/security/`。
