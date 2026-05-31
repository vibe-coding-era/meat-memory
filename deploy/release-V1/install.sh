#!/usr/bin/env bash

set -euo pipefail

REPO="${MEAT_MEMORY_REPO:-vibe-coding-era/meat-memory}"
VERSION="${MEAT_MEMORY_VERSION:-release-V1}"
RELEASE_BASE_URL="${MEAT_MEMORY_RELEASE_BASE_URL:-https://github.com/${REPO}/releases/download/${VERSION}}"
INSTALL_DIR="${MEAT_MEMORY_INSTALL_DIR:-${HOME}/.local/bin}"
CONFIG_DIR="${MEAT_MEMORY_CONFIG_DIR:-${HOME}/.config/meat-memory}"

log() {
  printf '[meat-memory] %s\n' "$*"
}

fail() {
  printf '[meat-memory] ERROR: %s\n' "$*" >&2
  exit 1
}

need_command() {
  command -v "$1" >/dev/null 2>&1 || fail "missing required command: $1"
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

download_file() {
  local url="$1"
  local output="$2"

  if command -v curl >/dev/null 2>&1; then
    curl -fL --retry 3 --connect-timeout 15 --output "$output" "$url"
  elif command -v wget >/dev/null 2>&1; then
    wget -O "$output" "$url"
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

main() {
  need_command uname
  need_command tar
  need_command awk
  need_command install

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
  archive_path="${tmp_dir}/${archive_name}"
  checksum_path="${archive_path}.sha256"

  trap 'rm -rf "${tmp_dir}"' EXIT

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
    if [ ! -f "${CONFIG_DIR}/app.toml" ]; then
      install -m 0644 "${bundle_dir}/config/app.toml" "${CONFIG_DIR}/app.toml"
      log "created config: ${CONFIG_DIR}/app.toml"
    else
      log "kept existing config: ${CONFIG_DIR}/app.toml"
    fi
  fi

  "${INSTALL_DIR}/memory-cli" --help >/dev/null

  log "installed memory-cli, memory-app, memory-worker"
  log "verification passed: ${INSTALL_DIR}/memory-cli --help"
  log "add ${INSTALL_DIR} to PATH if commands are not found"
}

main "$@"
