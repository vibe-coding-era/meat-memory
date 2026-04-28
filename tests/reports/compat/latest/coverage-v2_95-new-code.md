# V2.95 New Code Coverage Gate

Status: passed

Required: 100% targeted coverage for V2.95新增功能.

Covered regions:

- capability mapping
- adapter fixture parsing
- connector skeleton report
- CLI compat report
- HTTP V2.9 read-only surface
- MCP V2.9 read-only surface
- CLI / HTTP / MCP surface parity

Evidence:

- `cargo test -p memory-kernel --lib v29_compat`
- `cargo test -p memory-cli --bin memory-cli compat`
- `cargo test -p memory-http --lib v29`
- `cargo test -p memory-mcp --lib v29`
- `cargo test -p memory-cli --bin memory-cli surface_parity_smoke_covers_mcp_cli_and_http_contracts`
