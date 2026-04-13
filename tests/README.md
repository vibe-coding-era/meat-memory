# 测试目录

`tests/` 用来组织仓库级测试入口、验收语料和报告目录约定。

## 目录说明

- `unit/`：仓库级单元测试说明与补充入口
- `integration/`：跨 crate、数据库和服务级联动测试
- `e2e/`：CLI、HTTP、MCP 等完整链路验收
- `perf/`：性能和基准烟测
- `security/`：安全专项回归与说明
- `reports/`：测试报告落盘目录

## 推荐查看顺序

1. 先看 [`reports/README.md`](reports/README.md) 了解报告在哪里。
2. 再按需要进入 `unit`、`integration`、`e2e`、`perf`、`security`。

## 重点文件

| 目录 / 文件 | 用途 |
| --- | --- |
| `integration/v1-zh-acceptance.md` | V1 中文系统化验收语料与回归入口 |
| `integration/v2-bilingual-acceptance.md` | V2 双语验收说明 |
| `reports/latest-run.md` | 最近一次综合测试记录 |
