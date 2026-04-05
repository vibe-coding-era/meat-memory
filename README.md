# Meat Memory

Meat Memory is a self-hostable long-term memory kernel for agents, models, teams, and local/cloud hybrid deployments.

Current repository status:

- Architecture has been defined in [docs/meat-memory-scheme-v1.md](docs/meat-memory-scheme-v1.md) and [docs/meat-memory-scheme-v2.md](docs/meat-memory-scheme-v2.md).
- Execution planning lives in [tasks/tasklist.md](tasks/tasklist.md).
- Execution context is tracked in [tasks/task-log.md](tasks/task-log.md).
- The Rust workspace and bootstrap baseline are now initialized.
- V1 user-facing docs now cover HTTP/CLI/MCP, agent integration, local deploy, cloud deploy, and acceptance.

## Workspace Layout

```text
crates/
  memory-domain/
  memory-core/
  memory-kernel/
  memory-store/
  memory-store-pg/
  memory-store-md/
  memory-assets/
  memory-index/
  memory-sync/
  memory-policy/
  memory-extract/
  memory-models/
  memory-mcp/
  memory-http/
  memory-worker/
  memory-config/
  memory-observability/
  memory-cli/
  memory-app/
config/
scripts/
tests/
docs/
tasks/
```

## Quick Start

1. Verify the macOS toolchain and local dependencies.
2. Bootstrap Rust components and project-local helper tools.
3. Run the workspace checks.
4. Start the local development stack when PostgreSQL-backed flows are needed.

```bash
cp .env.example .env
./scripts/verify.sh
./scripts/bootstrap.sh
cargo check
cargo test
./scripts/dev-up.sh
```

If you use `just`, the same flows are available as:

```bash
just verify
just bootstrap
just test
just dev-up
```

After the stack is up, useful endpoints are:

```bash
curl http://127.0.0.1:8080/healthz
curl http://127.0.0.1:8080/api/v1/meta
curl http://127.0.0.1:8080/mcp/tools
```

## V1 Docs

- [docs/agent-integration-v1.md](docs/agent-integration-v1.md)
- [docs/api/http-api-v1.md](docs/api/http-api-v1.md)
- [docs/api/mcp-tools-v1.md](docs/api/mcp-tools-v1.md)
- [docs/api/cli-v1.md](docs/api/cli-v1.md)
- [docs/runbook/local-deploy-v1.md](docs/runbook/local-deploy-v1.md)
- [docs/runbook/cloud-deploy-v1.md](docs/runbook/cloud-deploy-v1.md)
- [docs/runbook/v1-acceptance.md](docs/runbook/v1-acceptance.md)

## Local Development Notes

- The project currently assumes a local Markdown store rooted at `./docs`.
- For vector-enabled PostgreSQL development, prefer the project-local pgvector container instead of mutating any existing machine-wide PostgreSQL instance.
- Configuration starts from `config/default.toml`. Copy `config/local.example.toml` to `config/local.toml` for machine-specific overrides.
- Environment variables are documented in `.env.example`.
- Local Git hooks can be installed with `./scripts/install-hooks.sh` and enforce `fmt`/`clippy`/`test` smoke plus Conventional Commits.
- `Cargo.lock` is committed for this application workspace and should be updated intentionally as part of dependency management work, not incidentally during unrelated feature changes.
- `memory-app` now exposes HTTP API plus optional MCP HTTP routes when `enable_mcp = true`.

## Near-Term Roadmap

- Expand kernel orchestration beyond the minimal remember/search path
- Add richer Markdown rollups and agent projections
- Introduce policy review/redaction and extraction pipeline stages
- Implement publish/sync flows and stronger retrieval planning
- Close V1 deployment, QA, and documentation gaps
