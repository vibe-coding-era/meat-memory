# Test Reports

This directory stores runnable test outputs grouped by category.

- `unit/`: workspace unit-style runs such as `cargo test --workspace --lib --bins`
- `integration/`: cross-crate and service integration runs for V2 scope/promotion/sync flows
- `e2e/`: CLI、安装后初始化、skill 导出等完整使用链路的验收输出
- `perf/`: persisted benchmark smoke outputs such as kernel and sync latency snapshots
- `security/`: reserved for executable security checks once they are wired in

Each category keeps:

- `latest/`: the newest generated report files
- `archive/`: timestamped copies from previous runs

Generate or refresh all reports with:

```bash
./scripts/write-test-reports.sh
```
