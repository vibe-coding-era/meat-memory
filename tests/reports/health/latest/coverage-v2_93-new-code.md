# V2.93 New Code Coverage Gate

Coverage target: 100% line / function / region coverage for V2.93 new production code.

Covered production areas:

- memory-domain secret finding and health risk model
- memory-kernel secret / PII detector, ingest guard, recall guard, hard-delete redaction, health analyzer and report writer
- memory-cli health report parser and output helpers
- memory-observability V2.9 security and health metrics
- memory-store-pg 0011 secret finding / health report migration wiring

Required suites executed by this script:

- memory-domain security and ids unit tests
- memory-kernel v29_security and v29_trace tests
- memory-cli health parser/report/helper tests
- memory-observability security metrics test
- memory-store-pg secret migration contract test
- CLI smoke for redact, deny, search, and health report projection

Status: pass when this script exits 0.
