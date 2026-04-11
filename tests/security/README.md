# Security Tests

本目录用于承载 V2.3 的安全测试设计与执行说明，重点覆盖：

- authorization：敏感接口必须需要合法 key，并且不能越权跨 scope / key 管理
- hijack prevention：Bearer / `x-meat-memory-key` 伪造、匿名访问、枚举与提权路径
- injection boundaries：SQL、Markdown、路径、header、tool arguments、base64 等输入边界
- payload limits：body/query/prompt/image 等资源消耗型输入限制

当前执行入口：

- HTTP / 集成安全回归：`cargo test -p memory-http --test http_api_tests --quiet -- --test-threads=1`
- MCP / 敏感工具回归：`cargo test -p memory-mcp --test mcp_tools_tests --quiet -- --test-threads=1`
- 报告汇总：`./scripts/security-report.sh`

后续本目录会继续补更细粒度的专项攻击用例索引与说明。
