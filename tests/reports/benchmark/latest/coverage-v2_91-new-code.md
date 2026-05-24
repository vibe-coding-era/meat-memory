# V2.91 New Code Coverage Gate

Coverage target: 100% line / function / region coverage for V2.91 new production code.

Covered production areas:

- memory-domain benchmark model and benchmark IDs
- memory-kernel V2.91 benchmark runner, fixture, recall / latency / leakage / cost metrics, report writer
- memory-cli benchmark run/report/compare parser and output helpers
- memory-store-pg 0009 benchmark migration wiring

Required suites executed by this script:

- memory-domain benchmark and ids unit tests
- memory-kernel v29_benchmark unit/integration tests
- memory-cli benchmark parser/report/compare/helper tests
- memory-store-pg benchmark migration contract test
- V2.7 lifecycle compatibility smoke
- V2.8 governance compatibility smoke

Status: pass when this script exits 0.
