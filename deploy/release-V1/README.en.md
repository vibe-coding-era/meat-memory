# Meat Memory release-V1 Deployment Package

This directory is the deployment-only delivery folder for `release-V1`. It contains deployment documentation and the installer, but no source code.

Default Chinese version: [README.md](./README.md)

## Contents

| File | Purpose |
| --- | --- |
| `README.md` | Default Chinese installation and operations guide |
| `README.en.md` | English installation and operations guide |
| `DEPLOYMENT_PLAN.md` | release-V1 deployment plan, release flow, and rollback strategy |
| `.env.example` | Environment template for binary deployment |
| `install.sh` | macOS / Linux binary installer |
| `dev-up.sh` | Start local app / worker from release binaries |
| `dev-down.sh` | Stop local processes started by `dev-up.sh` |

## Installation

New users and server deployments must download the installer from the server. Do not assume the machine already has a source checkout or a local `deploy/release-V1` directory. Recommended one-line install:

```bash
curl -fsSL https://raw.githubusercontent.com/vibe-coding-era/meat-memory/release-V1/deploy/release-V1/install.sh | bash -s -- --skip-tui --verify-write
```

This downloads the `release-V1` binary for the current platform, installs `memory-cli`, `memory-app`, and `memory-worker`, writes a markdown-only quickstart config, and verifies the first memory write/search.

If you want to save and review the script first, use the equivalent expanded flow:

```bash
mkdir -p ~/meat-memory-release-V1
cd ~/meat-memory-release-V1
curl -fsSLO https://raw.githubusercontent.com/vibe-coding-era/meat-memory/release-V1/deploy/release-V1/install.sh
chmod +x install.sh
./install.sh --check
./install.sh --skip-tui --verify-write
```

If the check reports missing base tools such as `curl`, `tar`, `awk`, or `coreutils`, review the package-manager command printed by the installer first. Then let the installer try to install them:

```bash
./install.sh --install-deps
```

Do not pipe a remote script directly into `bash --install-deps`. If system dependencies are missing, save the script first, review the script and package-manager command, then explicitly run `./install.sh --install-deps`.

The installer downloads prebuilt binaries from the `release-V1` GitHub Release by default. A first install writes a markdown-only quickstart config under the user's home directory, so a local PostgreSQL server is not required for the first memory. Config and data live under `~/.config/meat-memory` and `~/.local/share/meat-memory`.

To verify the first memory write immediately after installation, add `--verify-write`:

```bash
./install.sh --skip-tui --verify-write
```

The installer uses the installed `memory-cli remember` command to write one quickstart memory, then verifies it with `memory-cli search`. After installation, it starts `memory-cli tui init --interactive` when an interactive terminal is available. In CI, automation, or binary-only installation, skip TUI:

```bash
./install.sh --skip-tui
```

The default install directory is `~/.local/bin`. If it is not on your `PATH`, add:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

## Server Deployment Quick Start

If you see either of these errors, you are likely running an old source-development command or treating repository-local paths as the new-user install path:

```text
cp: .env.example: No such file or directory
zsh: no such file or directory: ./docs/scripts/dev-up.sh
```

On a server, download the deployment helper scripts from GitHub raw. Do not assume a source directory exists:

```bash
mkdir -p ~/meat-memory-release-V1 && cd ~/meat-memory-release-V1 && BASE_URL=https://raw.githubusercontent.com/vibe-coding-era/meat-memory/release-V1/deploy/release-V1 && for f in install.sh .env.example dev-up.sh dev-down.sh; do curl -fsSLO "$BASE_URL/$f"; done && chmod +x install.sh dev-up.sh dev-down.sh && ./install.sh --skip-tui --verify-write && cp .env.example .env && ./dev-up.sh
```

`dev-up.sh` reads `.env` from this directory, runs `install.sh` when release binaries are missing, and starts `memory-app` and `memory-worker` in the background. Stop them with:

```bash
./dev-down.sh
```

The default `.env` points to PostgreSQL / pgvector on `127.0.0.1:5433`; update `MEAT_MEMORY_DATABASE_URL` first if your database runs elsewhere.
`dev-up.sh` checks the database TCP endpoint before starting services; set `MEAT_MEMORY_SKIP_DB_CHECK=1` only when you intentionally want to skip that preflight.
Environment variables passed on the command line take precedence over `.env`, for example:

```bash
MEAT_MEMORY_SKIP_DB_CHECK=1 ./dev-up.sh
```

Note: `./docs/scripts/dev-up.sh` is the Docker development entrypoint for the source repository. It is not the release-V1 binary deployment entrypoint.

## Configuration

| Environment variable | Default | Description |
| --- | --- | --- |
| `MEAT_MEMORY_REPO` | `vibe-coding-era/meat-memory` | GitHub repository |
| `MEAT_MEMORY_VERSION` | `release-V1` | GitHub Release tag |
| `MEAT_MEMORY_RELEASE_BASE_URL` | GitHub Release URL | Private mirror or internal artifact repository URL |
| `MEAT_MEMORY_INSTALL_DIR` | `$HOME/.local/bin` | Binary install directory |
| `MEAT_MEMORY_CONFIG_DIR` | `$HOME/.config/meat-memory` | Default config directory |
| `MEAT_MEMORY_DATA_DIR` | `$HOME/.local/share/meat-memory` | Quickstart data directory |
| `MEAT_MEMORY_MARKDOWN_ROOT` | `$MEAT_MEMORY_DATA_DIR/markdown` | Quickstart markdown memory directory |
| `MEAT_MEMORY_ASSETS_ROOT` | `$MEAT_MEMORY_DATA_DIR/assets` | Quickstart asset directory |
| `MEAT_MEMORY_KEY_STORE_PATH` | `$MEAT_MEMORY_DATA_DIR/keys/default-key.toml` | Quickstart key config path |
| `MEAT_MEMORY_SYNC_STATE_PATH` | `$MEAT_MEMORY_DATA_DIR/sync/state.json` | Quickstart sync state path |
| `MEAT_MEMORY_DEFAULT_ENABLE_PG` | `0` | Whether generated first-install config enables PostgreSQL |
| `MEAT_MEMORY_OVERWRITE_CONFIG` | `0` | Set to `1` to replace an existing `app.toml` |
| `MEAT_MEMORY_TUI_CONFIG` | `$MEAT_MEMORY_CONFIG_DIR/app.local.toml` | Recommended local config written by TUI |
| `MEAT_MEMORY_RUN_TUI` | `auto` | `auto` runs TUI with an interactive terminal; `1` forces it; `0` skips it |
| `MEAT_MEMORY_INSTALL_MISSING_DEPS` | `0` | `1` is equivalent to `--install-deps` |
| `MEAT_MEMORY_VERIFY_SCOPE_ID` | `scp_release_v1_quickstart` | Scope used by `--verify-write` |
| `MEAT_MEMORY_ENV_FILE` | `<deployment script directory>/.env` | Environment file used by `dev-up.sh` |

Example:

```bash
MEAT_MEMORY_INSTALL_DIR=/usr/local/bin \
MEAT_MEMORY_VERSION=release-V1 \
./install.sh --skip-tui --verify-write
```

Note: `--check` does not download or install release binaries, but it creates the install and config directories to verify writability.

## Supported Platforms

| OS | Architecture | Release asset |
| --- | --- | --- |
| macOS | arm64 | `meat-memory-darwin-arm64.tar.gz` |
| macOS | x86_64 | `meat-memory-darwin-amd64.tar.gz` |
| Linux | arm64 / aarch64 | `meat-memory-linux-arm64.tar.gz` |
| Linux | x86_64 / amd64 | `meat-memory-linux-amd64.tar.gz` |

Each binary package must be published with its matching `.sha256` file. The installer verifies the checksum before installing executables.

## Post-Install Verification

```bash
export MEAT_MEMORY_CONFIG="$HOME/.config/meat-memory/app.toml"
memory-cli --help
memory-cli config check
command -v memory-app
command -v memory-worker
```

Write and search the first memory:

```bash
export MEAT_MEMORY_CONFIG="$HOME/.config/meat-memory/app.toml"
memory-cli remember \
  --scope-id scp_release_v1_quickstart \
  --title "Release V1 install smoke" \
  --body "Meat Memory release-V1 installer wrote this quickstart memory." \
  --memory-kind procedure \
  --json

memory-cli search "quickstart memory" --scope-id scp_release_v1_quickstart --limit 5 --json
```

`memory-app` and `memory-worker` are long-running service entrypoints. For installation verification, first check that the binaries exist; start them through `dev-up.sh`, systemd, launchd, or your process manager.

When the default config directory is used, the installer copies the sample `app.toml` from the release bundle to `~/.config/meat-memory/app.toml` and rewrites it into a user-directory markdown-only quickstart config. Existing config files are never overwritten unless `MEAT_MEMORY_OVERWRITE_CONFIG=1` is set. If `memory-cli config check` reports warnings, the installer continues and guides you into TUI configuration; those warnings usually mean the local database, model routes, or paths are not configured yet, not that binary installation failed. TUI defaults to writing `~/.config/meat-memory/app.local.toml`; verify it with:

```bash
MEAT_MEMORY_CONFIG="$HOME/.config/meat-memory/app.local.toml" memory-cli config check
```

## Deployment Modes

Recommended `release-V1` deployment modes:

| Scenario | Recommended mode |
| --- | --- |
| Local trial or single-user CLI | Run `memory-cli` directly |
| Long-running HTTP / MCP service | Run `memory-app` under systemd, launchd, or another process manager |
| Background jobs | Run `memory-worker` as an independently managed process |

For production or team deployments:

- Run `memory-app` and `memory-worker` under a dedicated service account.
- Keep configuration and data directories outside the binary install directory.
- Inject secrets and database URLs through environment variables, a secret manager, or system credential storage.
- Confirm that all `release-V1` GitHub Release assets and checksums were produced by trusted CI.

## Upgrade and Rollback

To upgrade, rerun the installer with the target `MEAT_MEMORY_VERSION`. The installer overwrites binaries but never overwrites existing configuration.

To roll back, install the previous verified release tag:

```bash
MEAT_MEMORY_VERSION=<previous-release-tag> ./install.sh --skip-tui
```

To uninstall binaries:

```bash
rm -f ~/.local/bin/memory-cli ~/.local/bin/memory-app ~/.local/bin/memory-worker
```

Back up or remove configuration and data directories according to your deployment policy.

## Troubleshooting

### `cp: .env.example: No such file or directory`

Download the deployment helper file and confirm the current directory:

```bash
mkdir -p ~/meat-memory-release-V1
cd ~/meat-memory-release-V1
BASE_URL=https://raw.githubusercontent.com/vibe-coding-era/meat-memory/release-V1/deploy/release-V1
curl -fsSLO "$BASE_URL/.env.example"
cp .env.example .env
```

### `./docs/scripts/dev-up.sh: no such file or directory`

Do not run the source-development script from a deployment-only package. Download and use the server deployment entrypoint:

```bash
mkdir -p ~/meat-memory-release-V1
cd ~/meat-memory-release-V1
BASE_URL=https://raw.githubusercontent.com/vibe-coding-era/meat-memory/release-V1/deploy/release-V1
curl -fsSLO "$BASE_URL/install.sh"
curl -fsSLO "$BASE_URL/.env.example"
curl -fsSLO "$BASE_URL/dev-up.sh"
curl -fsSLO "$BASE_URL/dev-down.sh"
chmod +x install.sh dev-up.sh dev-down.sh
./dev-up.sh
```

## Release Checklist

Before publishing `release-V1`, confirm:

- The GitHub Release tag is `release-V1`.
- All four platform assets and matching `.sha256` files are available.
- `install.sh --check` passes on target platforms.
- The installer has been tested on every target platform included in the release scope.
- Post-install `memory-cli tui init --interactive` has been completed, or `--skip-tui` was explicitly used.
- Production credentials, release database, OCR/ASR/video providers, benchmark evidence, and other external gates have formal evidence or an approved release waiver.
- Only deployment documentation and installer files from this directory are committed to GitHub; no source code is included.
