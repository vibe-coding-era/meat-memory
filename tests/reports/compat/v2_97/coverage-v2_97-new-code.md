# V2.97 New Code Coverage Gate

Status: passed

Required: 100% targeted coverage for V2.97新增功能.

Covered regions:

- connector dry-run contract
- local-git dry-run report
- local-git sync-plan checkpoint
- markdown-docs dry-run
- markdown-docs sync-plan / conflict review projection
- markdown-docs explicit apply path
- source auto discovery for connector apply
- chat-export JSON parser
- chat-export import-draft projection
- chat-export explicit apply path
- connector proposal-queue projection
- connector proposal apply-plan projection
- connector queue confirmation token
- connector proposal confirmed executor
- markdown-docs proposal queue
- local-git proposal queue
- chat-export proposal queue
- chat-export proposal apply-plan
- CLI parser and command projections
- CLI confirmed connector executor
- HTTP connector dry-run endpoint
- HTTP connector sync-plan endpoint
- HTTP connector import-draft endpoint
- HTTP connector proposal-queue endpoint
- HTTP connector proposal apply-plan endpoint
- MCP connector dry-run tool
- MCP connector sync-plan tool
- MCP connector import-draft tool
- MCP connector proposal-queue tool
- MCP connector proposal apply-plan tool

Evidence:

- `cargo test -p memory-kernel --lib v297_`
- `cargo test -p memory-cli --bin memory-cli connector_`
- `cargo test -p memory-http --lib v297_http_connector_`
- `cargo test -p memory-mcp --lib v297_mcp_connector_`
- `cargo test -p memory-cli --bin memory-cli cli_command_functions_cover_pg_management_paths`
- `docs/scripts/v2_97-acceptance.sh`
