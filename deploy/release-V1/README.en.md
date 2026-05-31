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

The recommended flow is environment check, install, then TUI configuration:

```bash
bash deploy/release-V1/install.sh --check
bash deploy/release-V1/install.sh
```

If the check reports missing base tools such as `curl`, `tar`, `awk`, or `coreutils`, review the package-manager command printed by the installer first. Then let the installer try to install them:

```bash
bash deploy/release-V1/install.sh --install-deps
```

Remote install:

```bash
curl -fsSL https://raw.githubusercontent.com/vibe-coding-era/meat-memory/release-V1/deploy/release-V1/install.sh | bash -s -- --install-deps
```

The installer downloads prebuilt binaries from the `release-V1` GitHub Release by default. After installation, it starts `memory-cli tui init --interactive` when an interactive terminal is available. In CI, automation, or binary-only installation, skip TUI:

```bash
bash deploy/release-V1/install.sh --skip-tui
```

The default install directory is `~/.local/bin`. If it is not on your `PATH`, add:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

## Local Deployment Quick Start

If you see either of these errors, you are likely running an old source-development command or running from the wrong directory:

```text
cp: .env.example: No such file or directory
zsh: no such file or directory: ./docs/scripts/dev-up.sh
```

Use the self-contained release-V1 deployment entrypoint instead:

```bash
cd deploy/release-V1
cp .env.example .env
./dev-up.sh
```

`dev-up.sh` reads `.env` from this directory, runs `install.sh` when release binaries are missing, and starts `memory-app` and `memory-worker` in the background. Stop them with:

```bash
./dev-down.sh
```

The default `.env` points to PostgreSQL / pgvector on `127.0.0.1:5433`; update `MEAT_MEMORY_DATABASE_URL` first if your database runs elsewhere.

Note: `./docs/scripts/dev-up.sh` is the Docker development entrypoint for the source repository. It is not the release-V1 binary deployment entrypoint.

## Configuration

| Environment variable | Default | Description |
| --- | --- | --- |
| `MEAT_MEMORY_REPO` | `vibe-coding-era/meat-memory` | GitHub repository |
| `MEAT_MEMORY_VERSION` | `release-V1` | GitHub Release tag |
| `MEAT_MEMORY_RELEASE_BASE_URL` | GitHub Release URL | Private mirror or internal artifact repository URL |
| `MEAT_MEMORY_INSTALL_DIR` | `$HOME/.local/bin` | Binary install directory |
| `MEAT_MEMORY_CONFIG_DIR` | `$HOME/.config/meat-memory` | Default config directory |
| `MEAT_MEMORY_TUI_CONFIG` | `$MEAT_MEMORY_CONFIG_DIR/app.local.toml` | Recommended local config written by TUI |
| `MEAT_MEMORY_RUN_TUI` | `auto` | `auto` runs TUI with an interactive terminal; `1` forces it; `0` skips it |
| `MEAT_MEMORY_INSTALL_MISSING_DEPS` | `0` | `1` is equivalent to `--install-deps` |
| `MEAT_MEMORY_ENV_FILE` | `deploy/release-V1/.env` | Environment file used by `dev-up.sh` |

Example:

```bash
MEAT_MEMORY_INSTALL_DIR=/usr/local/bin \
MEAT_MEMORY_VERSION=release-V1 \
bash deploy/release-V1/install.sh
```

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
memory-cli --help
memory-app --help
memory-worker --help
memory-cli config check
```

When the default config directory is used, the installer copies the sample `app.toml` from the release bundle to `~/.config/meat-memory/app.toml`. Existing config files are never overwritten. TUI defaults to writing `~/.config/meat-memory/app.local.toml`; verify it with:

```bash
MEAT_MEMORY_CONFIG=~/.config/meat-memory/app.local.toml memory-cli config check
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
MEAT_MEMORY_VERSION=<previous-release-tag> bash deploy/release-V1/install.sh
```

To uninstall binaries:

```bash
rm -f ~/.local/bin/memory-cli ~/.local/bin/memory-app ~/.local/bin/memory-worker
```

Back up or remove configuration and data directories according to your deployment policy.

## Troubleshooting

### `cp: .env.example: No such file or directory`

Check your current directory. For release-V1 deployment:

```bash
cd deploy/release-V1
cp .env.example .env
```

### `./docs/scripts/dev-up.sh: no such file or directory`

Do not run the source-development script from a deployment-only package. Use:

```bash
cd deploy/release-V1
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
