# API 文档

本目录汇总对外命令和协议接口，适合在你已经理解项目定位后查具体调用方式。

## 阅读顺序

1. [`cli-v1.md`](cli-v1.md)：先熟悉本地命令、配置检查和初始化流程
2. [`http-api-v1.md`](http-api-v1.md)：再看 HTTP 路由、请求体和示例
3. [`mcp-tools-v1.md`](mcp-tools-v1.md)：最后看 MCP 工具面和 transport 用法

## 常见入口

- `config check`：检查配置是否完整
- `mcp info`：查看 MCP 暴露状态
- `tui init`：生成或引导初始化配置
- `skills export`：导出 Agent Skill 包

如果你是第一次接入本项目，建议先读 [`../runbook/usage-guide.md`](../runbook/usage-guide.md) 再回到这里查细节。
