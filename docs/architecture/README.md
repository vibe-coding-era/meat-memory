# 架构文档

本目录用于存放当前实现基线的架构说明、模块地图和后续补充图示。

## 推荐入口

- [`system-design.md`](system-design.md)：当前最适合作为维护者入口的系统架构设计
- [`../meat-memory-scheme-v3.md`](../meat-memory-scheme-v3.md)：架构演进与版本规划基线

## 这一层主要回答什么

- Rust workspace 为什么这样拆分
- CLI、HTTP、MCP 如何共用同一套内核
- PostgreSQL、Markdown、Assets 为什么并存
- 本地开发、Docker 和后续云部署如何复用配置模型

后续如果补充时序图、ER 图、crate map，建议也放在本目录统一维护。
