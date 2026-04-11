# Test Strategy Layout

- `unit/`: crate-local and module-level tests
- `integration/`: repository and service integration tests
- `e2e/`: transport-level end-to-end tests
- `perf/`: performance and benchmark scenarios
- `security/`: security-focused scenarios
- `reports/`: persisted test outputs grouped by category

| 目录 / 文件 | 用途 |
| --- | --- |
| `integration/v1-zh-acceptance.md` | V1 中文系统化验收语料与回归入口说明 |
| `reports/README.md` | 各类测试报告的落盘目录、latest/archive 约定与统一生成脚本 |
