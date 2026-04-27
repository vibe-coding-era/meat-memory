# V2.92 New Code Coverage Gate

Coverage target: 100% line / function / region coverage for V2.92 new production code.

Covered production areas:

- memory-domain recall trace, explanation and budget pack model
- memory-kernel traced search, failure classifier, budget pack and report writer
- memory-cli trace latest / inspect parser and output helpers
- memory-observability V2.9 recall metrics
- memory-store-pg 0010 recall trace migration wiring

Required suites executed by this script:

- memory-domain recall and ids unit tests
- memory-kernel v29_trace and v29_benchmark tests
- memory-cli trace parser/report/helper tests
- memory-observability recall metrics test
- memory-store-pg recall migration contract test

Status: pass when this script exits 0.
