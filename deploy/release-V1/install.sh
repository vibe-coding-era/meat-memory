#!/usr/bin/env bash

set -euo pipefail

REPO="${MEAT_MEMORY_REPO:-vibe-coding-era/meat-memory}"
VERSION="${MEAT_MEMORY_VERSION:-release-V1}"
RELEASE_BASE_URL="${MEAT_MEMORY_RELEASE_BASE_URL:-https://github.com/${REPO}/releases/download/${VERSION}}"
INSTALL_DIR="${MEAT_MEMORY_INSTALL_DIR:-${HOME}/.local/bin}"
CONFIG_DIR="${MEAT_MEMORY_CONFIG_DIR:-${HOME}/.config/meat-memory}"
DATA_DIR="${MEAT_MEMORY_DATA_DIR:-${HOME}/.local/share/meat-memory}"
MARKDOWN_ROOT="${MEAT_MEMORY_MARKDOWN_ROOT:-${DATA_DIR}/markdown}"
ASSETS_ROOT="${MEAT_MEMORY_ASSETS_ROOT:-${DATA_DIR}/assets}"
KEY_STORE_PATH="${MEAT_MEMORY_KEY_STORE_PATH:-${DATA_DIR}/keys/default-key.toml}"
SYNC_STATE_PATH="${MEAT_MEMORY_SYNC_STATE_PATH:-${DATA_DIR}/sync/state.json}"
DEFAULT_ENABLE_PG="${MEAT_MEMORY_DEFAULT_ENABLE_PG:-0}"
OVERWRITE_CONFIG="${MEAT_MEMORY_OVERWRITE_CONFIG:-0}"
INSTALL_MISSING_DEPS="${MEAT_MEMORY_INSTALL_MISSING_DEPS:-0}"
RUN_TUI="${MEAT_MEMORY_RUN_TUI:-auto}"
TUI_CONFIG="${MEAT_MEMORY_TUI_CONFIG:-${CONFIG_DIR}/app.local.toml}"
CHECK_ONLY=0
VERIFY_WRITE=0
VERIFY_SCOPE_ID="${MEAT_MEMORY_VERIFY_SCOPE_ID:-scp_release_v1_quickstart}"
VERIFY_TITLE="${MEAT_MEMORY_VERIFY_TITLE:-Release V1 install smoke}"
VERIFY_BODY="${MEAT_MEMORY_VERIFY_BODY:-Meat Memory release-V1 installer wrote this quickstart memory.}"
MISSING_COMMANDS=()
TMP_DIR_TO_CLEAN=""

log() {
  printf '[meat-memory] %s\n' "$*"
}

fail() {
  printf '[meat-memory] ERROR: %s\n' "$*" >&2
  exit 1
}

cleanup() {
  if [ -n "${TMP_DIR_TO_CLEAN:-}" ]; then
    rm -rf "$TMP_DIR_TO_CLEAN"
  fi
}

need_command() {
  command -v "$1" >/dev/null 2>&1 || fail "missing required command: $1"
}

usage() {
  cat <<'EOF'
Meat Memory release-V1 installer

Usage:
  install.sh [--check] [--install-deps] [--verify-write] [--tui|--skip-tui]

Options:
  --check         Only run environment checks; do not download or install.
  --install-deps  Try to install missing base tools with the system package manager.
  --verify-write  After installing, write and search one markdown-only quickstart memory.
  --tui           Run `memory-cli tui init --interactive` after installing.
  --skip-tui      Skip post-install TUI configuration.
  -h, --help      Show this help.

Environment:
  MEAT_MEMORY_INSTALL_MISSING_DEPS=1   Same as --install-deps.
  MEAT_MEMORY_DATA_DIR=<path>          Default: $HOME/.local/share/meat-memory.
  MEAT_MEMORY_MARKDOWN_ROOT=<path>     Default: $MEAT_MEMORY_DATA_DIR/markdown.
  MEAT_MEMORY_ASSETS_ROOT=<path>       Default: $MEAT_MEMORY_DATA_DIR/assets.
  MEAT_MEMORY_DEFAULT_ENABLE_PG=0|1    Default: 0 for local markdown-only quickstart.
  MEAT_MEMORY_OVERWRITE_CONFIG=1       Replace an existing app.toml.
  MEAT_MEMORY_VERIFY_SCOPE_ID=<id>     Default: scp_release_v1_quickstart.
  MEAT_MEMORY_RUN_TUI=auto|1|0         Default is auto.
  MEAT_MEMORY_TUI_CONFIG=<path>        Default: $MEAT_MEMORY_CONFIG_DIR/app.local.toml.
EOF
}

parse_args() {
  while [ "$#" -gt 0 ]; do
    case "$1" in
      --check)
        CHECK_ONLY=1
        shift
        ;;
      --install-deps)
        INSTALL_MISSING_DEPS=1
        shift
        ;;
      --verify-write)
        VERIFY_WRITE=1
        shift
        ;;
      --tui)
        RUN_TUI=1
        shift
        ;;
      --skip-tui|--no-tui)
        RUN_TUI=0
        shift
        ;;
      -h|--help)
        usage
        exit 0
        ;;
      *)
        fail "unknown argument: $1"
        ;;
    esac
  done
}

detect_asset_name() {
  local os
  local arch

  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os" in
    Darwin)
      os="darwin"
      ;;
    Linux)
      os="linux"
      ;;
    *)
      fail "unsupported operating system: ${os}"
      ;;
  esac

  case "$arch" in
    arm64|aarch64)
      arch="arm64"
      ;;
    x86_64|amd64)
      arch="amd64"
      ;;
    *)
      fail "unsupported architecture: ${arch}"
      ;;
  esac

  printf 'meat-memory-%s-%s' "$os" "$arch"
}

has_downloader() {
  command -v curl >/dev/null 2>&1 || command -v wget >/dev/null 2>&1
}

has_checksum_tool() {
  command -v sha256sum >/dev/null 2>&1 || command -v shasum >/dev/null 2>&1
}

collect_missing_commands() {
  MISSING_COMMANDS=()

  for cmd in uname tar awk install mktemp mkdir chmod grep sed; do
    if ! command -v "$cmd" >/dev/null 2>&1; then
      MISSING_COMMANDS+=("$cmd")
    fi
  done

  if ! has_downloader; then
    MISSING_COMMANDS+=("curl-or-wget")
  fi

  if ! has_checksum_tool; then
    MISSING_COMMANDS+=("sha256sum-or-shasum")
  fi
}

print_dependency_hint() {
  local os
  os="$(uname -s 2>/dev/null || printf unknown)"

  case "$os" in
    Darwin)
      log "install missing tools with Homebrew, for example: brew install curl coreutils gawk"
      ;;
    Linux)
      if command -v apt-get >/dev/null 2>&1; then
        log "install missing tools with: sudo apt-get update && sudo apt-get install -y ca-certificates curl coreutils tar gawk"
      elif command -v dnf >/dev/null 2>&1; then
        log "install missing tools with: sudo dnf install -y ca-certificates curl coreutils tar gawk"
      elif command -v yum >/dev/null 2>&1; then
        log "install missing tools with: sudo yum install -y ca-certificates curl coreutils tar gawk"
      elif command -v pacman >/dev/null 2>&1; then
        log "install missing tools with: sudo pacman -Sy --needed ca-certificates curl coreutils tar gawk"
      elif command -v apk >/dev/null 2>&1; then
        log "install missing tools with: sudo apk add ca-certificates curl coreutils tar gawk"
      else
        log "install missing tools with your OS package manager: ca-certificates curl coreutils tar gawk"
      fi
      ;;
    *)
      log "install missing tools with your OS package manager: curl or wget, tar, awk, coreutils, sha256 support"
      ;;
  esac
}

as_root() {
  if [ "$(id -u)" = "0" ]; then
    "$@"
  elif command -v sudo >/dev/null 2>&1; then
    sudo "$@"
  else
    return 1
  fi
}

install_missing_dependencies() {
  local os
  os="$(uname -s 2>/dev/null || printf unknown)"

  log "trying to install missing environment tools"

  case "$os" in
    Darwin)
      command -v brew >/dev/null 2>&1 || return 1
      brew install curl coreutils gawk
      ;;
    Linux)
      if command -v apt-get >/dev/null 2>&1; then
        as_root apt-get update
        as_root apt-get install -y ca-certificates curl coreutils tar gawk
      elif command -v dnf >/dev/null 2>&1; then
        as_root dnf install -y ca-certificates curl coreutils tar gawk
      elif command -v yum >/dev/null 2>&1; then
        as_root yum install -y ca-certificates curl coreutils tar gawk
      elif command -v pacman >/dev/null 2>&1; then
        as_root pacman -Sy --needed ca-certificates curl coreutils tar gawk
      elif command -v apk >/dev/null 2>&1; then
        as_root apk add ca-certificates curl coreutils tar gawk
      else
        return 1
      fi
      ;;
    *)
      return 1
      ;;
  esac
}

ensure_environment() {
  local asset_name

  log "checking environment"
  collect_missing_commands

  if [ "${#MISSING_COMMANDS[@]}" -gt 0 ]; then
    log "missing commands: ${MISSING_COMMANDS[*]}"
    if [ "$INSTALL_MISSING_DEPS" = "1" ]; then
      install_missing_dependencies || {
        print_dependency_hint
        fail "failed to install missing environment tools automatically"
      }
      collect_missing_commands
      if [ "${#MISSING_COMMANDS[@]}" -gt 0 ]; then
        log "still missing commands: ${MISSING_COMMANDS[*]}"
        print_dependency_hint
        fail "environment check failed after dependency installation"
      fi
    else
      print_dependency_hint
      fail "environment check failed; rerun with --install-deps after reviewing the package command"
    fi
  fi

  asset_name="$(detect_asset_name)"
  log "platform asset: ${asset_name}.tar.gz"

  mkdir -p "$INSTALL_DIR" "$CONFIG_DIR" "$MARKDOWN_ROOT" "$ASSETS_ROOT" "${DATA_DIR}/keys" "${DATA_DIR}/sync"
  [ -w "$INSTALL_DIR" ] || fail "install dir is not writable: ${INSTALL_DIR}"
  [ -w "$CONFIG_DIR" ] || fail "config dir is not writable: ${CONFIG_DIR}"
  [ -w "$MARKDOWN_ROOT" ] || fail "markdown root is not writable: ${MARKDOWN_ROOT}"
  [ -w "$ASSETS_ROOT" ] || fail "assets root is not writable: ${ASSETS_ROOT}"

  if ! printf ':%s:' "${PATH:-}" | grep -F ":${INSTALL_DIR}:" >/dev/null 2>&1; then
    log "PATH warning: ${INSTALL_DIR} is not on PATH"
  fi
}

download_file() {
  local url="$1"
  local output="$2"

  if command -v curl >/dev/null 2>&1; then
    curl -fL --retry 3 --connect-timeout 15 --output "$output" "$url" \
      || fail "failed to download ${url}; verify ${VERSION} release assets or MEAT_MEMORY_RELEASE_BASE_URL"
  elif command -v wget >/dev/null 2>&1; then
    wget -O "$output" "$url" \
      || fail "failed to download ${url}; verify ${VERSION} release assets or MEAT_MEMORY_RELEASE_BASE_URL"
  else
    fail "curl or wget is required to download release assets"
  fi
}

sha256_of() {
  local file="$1"

  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$file" | awk '{print $1}'
  else
    shasum -a 256 "$file" | awk '{print $1}'
  fi
}

verify_checksum() {
  local archive="$1"
  local checksum_file="$2"
  local expected
  local actual

  expected="$(awk 'NF {print $1; exit}' "$checksum_file")"
  [ -n "$expected" ] || fail "checksum file is empty: ${checksum_file}"

  actual="$(sha256_of "$archive")"
  [ "$actual" = "$expected" ] || fail "checksum mismatch for ${archive}"
}

install_binary() {
  local src="$1"
  local dest="$2"

  install -m 0755 "$src" "$dest"
}

toml_bool() {
  case "$1" in
    1|true|TRUE|yes|YES|on|ON)
      printf 'true'
      ;;
    *)
      printf 'false'
      ;;
  esac
}

toml_escape() {
  printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
}

write_release_config() {
  local src="$1"
  local dest="$2"
  local tmp="${dest}.tmp.$$"
  local markdown_root
  local assets_root
  local key_store_path
  local sync_state_path
  local enable_pg

  [ -f "$src" ] || fail "release bundle missing config/app.toml"

  markdown_root="$(toml_escape "$MARKDOWN_ROOT")"
  assets_root="$(toml_escape "$ASSETS_ROOT")"
  key_store_path="$(toml_escape "$KEY_STORE_PATH")"
  sync_state_path="$(toml_escape "$SYNC_STATE_PATH")"
  enable_pg="$(toml_bool "$DEFAULT_ENABLE_PG")"

  awk \
    -v markdown_root="$markdown_root" \
    -v assets_root="$assets_root" \
    -v key_store_path="$key_store_path" \
    -v sync_state_path="$sync_state_path" \
    -v enable_pg="$enable_pg" '
      /^\[[^]]+\]$/ { section = $0 }
      section == "[markdown]" && $0 ~ /^root[[:space:]]*=/ {
        print "root = \"" markdown_root "\""
        next
      }
      section == "[assets]" && $0 ~ /^root[[:space:]]*=/ {
        print "root = \"" assets_root "\""
        next
      }
      section == "[access]" && $0 ~ /^key_store_path[[:space:]]*=/ {
        print "key_store_path = \"" key_store_path "\""
        next
      }
      section == "[sync]" && $0 ~ /^mode[[:space:]]*=/ {
        print "mode = \"local_only\""
        if (!sync_state_written) {
          print "state_path = \"" sync_state_path "\""
          sync_state_written = 1
        }
        next
      }
      section == "[sync]" && $0 ~ /^state_path[[:space:]]*=/ {
        if (!sync_state_written) {
          print "state_path = \"" sync_state_path "\""
          sync_state_written = 1
        }
        next
      }
      section == "[features]" && $0 ~ /^enable_pg[[:space:]]*=/ {
        print "enable_pg = " enable_pg
        next
      }
      section == "[features]" && $0 ~ /^enable_markdown[[:space:]]*=/ {
        print "enable_markdown = true"
        next
      }
      { print }
    ' "$src" > "$tmp"

  install -m 0644 "$tmp" "$dest"
  rm -f "$tmp"
}

has_interactive_tty() {
  [ -r /dev/tty ] && [ -w /dev/tty ]
}

run_tui_config() {
  local config_path="${CONFIG_DIR}/app.toml"
  local tui_args=(
    "tui"
    "init"
    "--interactive"
    "--write-config"
    "$TUI_CONFIG"
  )

  case "$RUN_TUI" in
    0|false|no)
      log "skipped TUI configuration"
      log "run later: MEAT_MEMORY_CONFIG=${config_path} ${INSTALL_DIR}/memory-cli tui init --interactive --write-config ${TUI_CONFIG}"
      return
      ;;
    auto)
      if ! has_interactive_tty; then
        log "no interactive TTY detected; skipped TUI configuration"
        log "run later: MEAT_MEMORY_CONFIG=${config_path} ${INSTALL_DIR}/memory-cli tui init --interactive --write-config ${TUI_CONFIG}"
        return
      fi
      ;;
    1|true|yes)
      has_interactive_tty || fail "--tui requires an interactive terminal"
      ;;
    *)
      fail "invalid MEAT_MEMORY_RUN_TUI value: ${RUN_TUI}"
      ;;
  esac

  log "starting TUI configuration"
  log "recommended output config: ${TUI_CONFIG}"

  if [ -f "$TUI_CONFIG" ]; then
    log "existing TUI config will not be overwritten unless you confirm --force inside the wizard"
  fi

  MEAT_MEMORY_CONFIG="$config_path" "${INSTALL_DIR}/memory-cli" "${tui_args[@]}" </dev/tty >/dev/tty

  if [ -f "$TUI_CONFIG" ]; then
    log "TUI config written: ${TUI_CONFIG}"
    log "verify with: MEAT_MEMORY_CONFIG=${TUI_CONFIG} ${INSTALL_DIR}/memory-cli config check"
  else
    log "TUI finished without writing ${TUI_CONFIG}"
  fi
}

verify_memory_write() {
  local config_path="${CONFIG_DIR}/app.toml"
  local query="quickstart memory"
  local verify_dir
  local remember_output
  local search_output

  [ -x "${INSTALL_DIR}/memory-cli" ] || fail "memory-cli is not installed at ${INSTALL_DIR}/memory-cli"
  [ -f "$config_path" ] || fail "config file is missing: ${config_path}"

  verify_dir="$(mktemp -d "${TMPDIR:-/tmp}/meat-memory-verify.XXXXXX")"
  remember_output="${verify_dir}/remember.json"
  search_output="${verify_dir}/search.json"

  log "verifying first memory write"
  MEAT_MEMORY_CONFIG="$config_path" \
  MEAT_MEMORY_ENABLE_PG=0 \
  MEAT_MEMORY_ENABLE_MARKDOWN=1 \
  MEAT_MEMORY_MARKDOWN_ROOT="$MARKDOWN_ROOT" \
  MEAT_MEMORY_ASSETS_ROOT="$ASSETS_ROOT" \
    "${INSTALL_DIR}/memory-cli" remember \
      --scope-id "$VERIFY_SCOPE_ID" \
      --title "$VERIFY_TITLE" \
      --body "$VERIFY_BODY" \
      --memory-kind procedure \
      --json > "$remember_output"

  grep -q '"wrote_markdown"[[:space:]]*:[[:space:]]*true' "$remember_output" \
    || fail "memory write verification did not report wrote_markdown=true"

  MEAT_MEMORY_CONFIG="$config_path" \
  MEAT_MEMORY_ENABLE_PG=0 \
  MEAT_MEMORY_ENABLE_MARKDOWN=1 \
  MEAT_MEMORY_MARKDOWN_ROOT="$MARKDOWN_ROOT" \
  MEAT_MEMORY_ASSETS_ROOT="$ASSETS_ROOT" \
    "${INSTALL_DIR}/memory-cli" search "$query" \
      --scope-id "$VERIFY_SCOPE_ID" \
      --limit 5 \
      --json > "$search_output"

  grep -Eq '"memory_count"[[:space:]]*:[[:space:]]*[1-9]' "$search_output" \
    || fail "memory search verification did not find the quickstart memory"

  log "memory write verification passed"
  log "markdown root: ${MARKDOWN_ROOT}"
  rm -rf "$verify_dir"
}

main() {
  parse_args "$@"
  ensure_environment

  if [ "$CHECK_ONLY" = "1" ]; then
    log "environment check passed"
    exit 0
  fi

  local asset_name
  local archive_name
  local tmp_dir
  local archive_path
  local checksum_path
  local bundle_root
  local bundle_dir
  local binary

  asset_name="$(detect_asset_name)"
  archive_name="${asset_name}.tar.gz"
  tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/meat-memory-install.XXXXXX")"
  TMP_DIR_TO_CLEAN="$tmp_dir"
  archive_path="${tmp_dir}/${archive_name}"
  checksum_path="${archive_path}.sha256"

  trap cleanup EXIT

  log "repository: ${REPO}"
  log "release: ${VERSION}"
  log "asset: ${archive_name}"
  log "install dir: ${INSTALL_DIR}"

  download_file "${RELEASE_BASE_URL}/${archive_name}" "$archive_path"
  download_file "${RELEASE_BASE_URL}/${archive_name}.sha256" "$checksum_path"
  verify_checksum "$archive_path" "$checksum_path"

  tar -xzf "$archive_path" -C "$tmp_dir"
  bundle_root="$(tar -tzf "$archive_path" | awk -F/ 'NF {print $1; exit}')"
  bundle_dir="${tmp_dir}/${bundle_root}"

  [ -d "${bundle_dir}/bin" ] || fail "release bundle missing bin directory"

  mkdir -p "$INSTALL_DIR"

  for binary in memory-cli memory-app memory-worker; do
    [ -f "${bundle_dir}/bin/${binary}" ] || fail "release bundle missing ${binary}"
    install_binary "${bundle_dir}/bin/${binary}" "${INSTALL_DIR}/${binary}"
  done

  if [ -f "${bundle_dir}/config/app.toml" ]; then
    mkdir -p "$CONFIG_DIR"
    if [ ! -f "${CONFIG_DIR}/app.toml" ] || [ "$(toml_bool "$OVERWRITE_CONFIG")" = "true" ]; then
      write_release_config "${bundle_dir}/config/app.toml" "${CONFIG_DIR}/app.toml"
      log "created config: ${CONFIG_DIR}/app.toml"
      log "quickstart storage: ${DATA_DIR}"
      log "quickstart mode: markdown-only; set MEAT_MEMORY_DEFAULT_ENABLE_PG=1 for PostgreSQL"
    else
      log "kept existing config: ${CONFIG_DIR}/app.toml"
    fi
  fi

  "${INSTALL_DIR}/memory-cli" --help >/dev/null

  log "installed memory-cli, memory-app, memory-worker"
  log "verification passed: ${INSTALL_DIR}/memory-cli --help"
  MEAT_MEMORY_CONFIG="${CONFIG_DIR}/app.toml" "${INSTALL_DIR}/memory-cli" config check >/dev/null || log "config check reported warnings; use TUI configuration to review settings"
  if [ "$VERIFY_WRITE" = "1" ]; then
    verify_memory_write
  fi
  run_tui_config
  log "add ${INSTALL_DIR} to PATH if commands are not found"
}

main "$@"
