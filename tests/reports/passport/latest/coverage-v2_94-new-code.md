# V2.94 New Code Coverage Gate

Coverage target: 100% line / function / region coverage for V2.94 new production code.

Covered production areas:

- memory-domain evidence span model and passport manifest/hash model
- memory-kernel evidence derivation, provenance query, passport export/import/verify and report writer
- memory-cli passport export / verify / import / provenance parser and output helpers
- memory-store-pg 0012 evidence span / passport metadata migration wiring

Required suites executed by this script:

- memory-domain evidence, passport and ids unit tests
- memory-kernel v29_passport tests
- memory-cli passport parser/report/helper tests
- memory-store-pg passport migration contract test
- CLI smoke for provenance, export, verify, import, redaction and imported search

Status: pass when this script exits 0.
