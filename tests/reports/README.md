# 测试报告目录

本目录统一存放测试输出，方便在本地回看、归档和对比。

## 分类

- `unit/`：`cargo test --workspace --lib --bins` 这类工作区单测输出
- `integration/`：跨 crate、数据库和服务集成输出
- `e2e/`：CLI、初始化、skill 导出、V2.4 文档与上下文链路、V2.8 治理闭环验收输出
- `perf/`：性能烟测与延迟快照
- `security/`：安全专项报告

## 目录约定

每个分类目录都尽量保持两层结构：

- `latest/`：最近一次生成结果
- `archive/`：带时间戳的历史归档

## 生成方式

```bash
./docs/scripts/write-test-reports.sh
```

安全专项可以单独执行：

```bash
./docs/scripts/security-report.sh
```
