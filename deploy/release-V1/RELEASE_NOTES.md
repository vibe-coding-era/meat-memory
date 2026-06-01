# Meat Memory release-V1

## Install

Use the platform binary asset (`meat-memory-<os>-<arch>.tar.gz`) or the one-line installer below.

```bash
curl -fsSL https://raw.githubusercontent.com/vibe-coding-era/meat-memory/release-V1/deploy/release-V1/install.sh | bash -s -- --skip-tui --verify-write
```

Do not use GitHub auto-generated `Source code (zip)` or `Source code (tar.gz)` as install packages. They are source snapshots.

## Server Helpers

For server deployment helpers, download `install.sh`, `.env.example`, `dev-up.sh`, and `dev-down.sh` from `deploy/release-V1`.

The default `.env.example` is markdown-only (`MEAT_MEMORY_ENABLE_PG=0`), so PostgreSQL / pgvector is only required when explicitly enabled with `MEAT_MEMORY_ENABLE_PG=1`.

## Assets

- `meat-memory-darwin-arm64.tar.gz`
- `meat-memory-darwin-amd64.tar.gz`
- `meat-memory-linux-amd64.tar.gz`
- `meat-memory-linux-arm64.tar.gz`

Each tarball has a matching `.sha256` file.
