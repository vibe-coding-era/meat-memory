# Performance Tests

Use `./scripts/v2-perf.sh` to run the current V2 smoke benchmarks.

The script executes ignored Rust tests that print JSON summaries for:

- `memory-kernel` markdown-only `remember/browse/promote` latency and throughput
- `memory-sync` file-backed `append/pull/apply` latency and throughput

Default reports are written to:

- `tests/reports/perf/latest/kernel-perf.txt`
- `tests/reports/perf/latest/sync-perf.txt`

Run `./scripts/write-test-reports.sh` to refresh unit/integration/e2e/perf reports together.
