# 测试方案

本目录作为技术文档中的“测试方案”总入口，统一说明测试分层、验收语料、报告位置和回归入口。

## 怎么使用

如果你的目标是“先知道该跑什么，再知道报告在哪”，推荐按下面顺序操作：

```bash
./docs/scripts/test-required.sh
./docs/scripts/write-test-reports.sh
```

然后查看：

```bash
cat tests/reports/latest-run.md
ls tests/reports/e2e/latest
ls tests/reports/perf/latest
```

## 适合什么时候看

- 提交前想知道应该跑哪些测试
- 想区分 unit / integration / e2e / perf / security 的职责
- 想知道验收语料和测试报告分别放在哪里

## 推荐阅读顺序

1. [`../../tests/README.md`](../../tests/README.md)：测试目录总体说明
2. [`../../tests/reports/README.md`](../../tests/reports/README.md)：测试报告位置与生成方式
3. [`../runbook/v1-acceptance.md`](../runbook/v1-acceptance.md)：验收标准和入口

## 测试分层

- [`../../tests/unit/README.md`](../../tests/unit/README.md)：仓库级单元测试说明
- [`../../tests/integration/README.md`](../../tests/integration/README.md)：跨 crate / 数据库 / 服务集成测试
- [`../../tests/e2e/README.md`](../../tests/e2e/README.md)：CLI / HTTP / MCP 全链路验收
- [`../../tests/perf/README.md`](../../tests/perf/README.md)：性能烟测与 benchmark 入口
- [`../../tests/security/README.md`](../../tests/security/README.md)：安全专项测试说明

## 验收与语料

- [`../runbook/v1-acceptance.md`](../runbook/v1-acceptance.md)：V1 验收标准与入口
- [`../../tests/integration/v1-zh-acceptance.md`](../../tests/integration/v1-zh-acceptance.md)：V1 中文验收语料
- [`../../tests/integration/v2-bilingual-acceptance.md`](../../tests/integration/v2-bilingual-acceptance.md)：V2 双语验收说明
- [`../V2.8/V2.8.8-acceptance.md`](../V2.8/V2.8.8-acceptance.md)：V2.8 proposal-first 治理验收入口与场景

## 报告与分析

- [`../../tests/reports/README.md`](../../tests/reports/README.md)：测试输出的 `latest/archive` 约定
- [`../../tests/reports/latest-run.md`](../../tests/reports/latest-run.md)：最近一次综合测试记录
- [`../reports/README.md`](../reports/README.md)：分析型测试/覆盖率报告入口
- [`../reports/test-coverage-report.md`](../reports/test-coverage-report.md)：覆盖率分析报告

## 常用入口

- `./docs/scripts/test-required.sh`
- `./docs/scripts/v1-acceptance.sh`
- `./docs/scripts/v2_1-acceptance.sh`
- `./docs/scripts/v2_4-acceptance.sh`
- `./docs/scripts/v2_8-acceptance.sh`
- `./docs/scripts/v2_8-production-gate.sh`
- `./docs/scripts/write-test-reports.sh`
- `./docs/scripts/security-report.sh`
- `./docs/scripts/v2-perf.sh`

## 例子

### 例子 1：提交前最小测试检查

```bash
./docs/scripts/test-required.sh
```

适合：

- 提交前快速确认仓库没有明显回归

### 例子 2：刷新整套测试报告

```bash
./docs/scripts/write-test-reports.sh
cat tests/reports/latest-run.md
```

适合：

- 需要整理最近一次测试输出
- 需要把 `latest/archive` 报告一起更新

### 例子 3：只看性能与安全

```bash
./docs/scripts/v2-perf.sh
./docs/scripts/security-report.sh
```

适合：

- 不想全量回归，只想检查专项表现

## 维护约定

- 新增测试类型时，优先补 `tests/` 目录 README 与本页导航
- 新增验收脚本时，同时补 `tests/e2e/README.md` 和本页“常用入口”
- 静态分析报告放 `docs/reports/`，脚本生成结果放 `tests/reports/`
