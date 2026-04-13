# Performance Tests

本目录说明当前性能烟测的入口和报告位置。

运行方式：

```bash
./scripts/v2-perf.sh
```

该脚本会执行被标记为 ignored 的 Rust 性能测试，当前重点覆盖：

- `memory-kernel` 的 `remember / browse / promote`
- `memory-sync` 的 `append / pull / apply`

默认报告写入：

- `tests/reports/perf/latest/kernel-perf.txt`
- `tests/reports/perf/latest/sync-perf.txt`

如果想连同其他测试报告一起刷新，可执行：

```bash
./scripts/write-test-reports.sh
```
