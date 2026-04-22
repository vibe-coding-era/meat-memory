# V2.7 Acceptance Latest

## Scope

V2.7 acceptance validates the lifecycle system goal:

- Normalize short-term, mid-term, and long-term memory into lifecycle-aware records.
- Preserve source metadata for explainability.
- Apply recall governance for archived, deprecated, forgotten, restricted, and expired records.
- Expose governance through HTTP, MCP, and CLI entrypoints.
- Keep MCP, CLI, HTTP API, and TUI responsibilities explicit so lifecycle capabilities do not drift across surfaces.
- Produce memory health reporting and lifecycle metrics.

## Entry

Run:

```bash
./docs/scripts/v2_7-acceptance.sh
```

## Covered Checks

- `memory-domain`: unified lifecycle status and record mapping.
- `memory-observability`: V2.7 lifecycle counters.
- `memory-store-pg`: `source_refs` and lifecycle audit migration.
- `memory-store-md`: frontmatter/parser roundtrip for lifecycle metadata.
- `memory-kernel`: inspect, explanation, forget, restore, report, audit, and recall exclusion.
- `memory-http`: lifecycle routes and metrics surface.
- `memory-mcp`: lifecycle tool listing and dispatch surface.
- `memory-cli`: lifecycle command parsing and runtime support.
- `memory-cli`: MCP / CLI / HTTP API surface parity smoke for V2.7 core entrypoints.
- `cargo check --workspace --all-targets`: interface consistency.

## Current Status

Local targeted validation passed on 2026-04-19:

- `cargo test -p memory-domain -p memory-observability -p memory-store-md -p memory-store-pg -p memory-kernel`
- `cargo test -p memory-http -p memory-mcp -p memory-cli`

Full script entry is available at `docs/scripts/v2_7-acceptance.sh`.

Full V2.7 acceptance script passed on 2026-04-19 after allowing access to the local PostgreSQL test database:

- `./docs/scripts/v2_7-acceptance.sh`

Additional endpoint-level regressions are included for:

- HTTP lifecycle inspect / forget / restore / report / audit.
- MCP lifecycle inspect / forget / restore / report dispatch.
- CLI lifecycle status parsing and output payload helpers.
- MCP / CLI / HTTP API / TUI capability and parameter parity is documented in `docs/V2.7/V2.7.5-surface-parity.md`; the automated smoke passed in `cargo test -p memory-cli -- --test-threads=1` on 2026-04-20.

Additional CLI coverage boost on 2026-04-20:

- `cargo test -p memory-cli -- --test-threads=1` passed with 44 unit tests and 8 e2e tests.
- Focused CLI coverage artifact: `target/coverage/v2_7-cli-20260420-212000/report.txt`.
- `crates/memory-cli/src/main.rs`: Line `100.00%`, Function `100.00%`, Region `100.00%`.
- `crates/memory-cli/src/app.rs`: Line `89.97%`, Function `79.59%`, Region `75.11%`.

Additional CLI stabilization on 2026-04-21:

- `cargo test -p memory-kernel --lib lifecycle -- --test-threads=1` passed with 20 lifecycle tests, covering the non-ASCII truncation, memory source metadata, and restricted recall authorization regressions.
- `cargo test -p memory-kernel --lib lifecycle -- --test-threads=1` passed with 22 lifecycle tests after strengthening P1 review regression coverage.
- `cargo test -p memory-cli -- --test-threads=1` passed with 47 unit tests and 8 e2e tests.
- CLI entry dispatch now has a direct `run_with_cli(cli)` seam, keeping `run()` responsible only for process argument parsing and making static commands testable without runtime bootstrap.
- PG-backed CLI command/e2e tests now check local PostgreSQL availability before executing the PG path, so default local runs no longer fail or wait for pool timeout when `127.0.0.1:5433` is unavailable.
- Focused no-PG CLI coverage artifact: `target/coverage/v2_7-cli-20260421-152207/report-with-bin.txt`.
- No-PG coverage is not directly comparable with the PG-backed 2026-04-20 snapshot because PG command/e2e bodies are intentionally skipped in this environment.
- `crates/memory-cli/src/main.rs`: Line `100.00%`, Function `100.00%`, Region `100.00%`.
- `crates/memory-cli/src/app.rs` under no-PG fallback: Line `59.90%`, Function `55.51%`, Region `44.55%`.

Production gate added on 2026-04-21:

- Local gate entry: `./docs/scripts/v2_7-production-gate.sh`.
- Strict release gate: `V2_7_GATE_STRICT=1 V2_7_GATE_COVERAGE=1 ./docs/scripts/v2_7-production-gate.sh`.
- Gate design is documented in `docs/V2.7/V2.7.6-production-quality-gate.md`.
