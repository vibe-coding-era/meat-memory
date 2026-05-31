#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ENV_FILE="${MEAT_MEMORY_ENV_FILE:-${SCRIPT_DIR}/.env}"

log() {
  printf '[meat-memory] %s\n' "$*"
}

fail() {
  printf '[meat-memory] ERROR: %s\n' "$*" >&2
  exit 1
}

abspath_from_script_dir() {
  case "$1" in
    /*)
      printf '%s\n' "$1"
      ;;
    *)
      printf '%s/%s\n' "$SCRIPT_DIR" "$1"
      ;;
  esac
}

is_running() {
  local pid_file="$1"
  [ -f "$pid_file" ] || return 1
  local pid
  pid="$(cat "$pid_file")"
  [ -n "$pid" ] && kill -0 "$pid" >/dev/null 2>&1
}

start_process() {
  local name="$1"
  local binary="$2"
  local pid_file="${SCRIPT_DIR}/run/${name}.pid"
  local log_file="${SCRIPT_DIR}/logs/${name}.log"

  if is_running "$pid_file"; then
    log "${name} already running with pid $(cat "$pid_file")"
    return
  fi

  log "starting ${name}"
  nohup "$binary" >>"$log_file" 2>&1 &
  local pid=$!
  echo "$pid" >"$pid_file"

  sleep 1
  if ! kill -0 "$pid" >/dev/null 2>&1; then
    tail -n 40 "$log_file" >&2 || true
    fail "${name} failed to start; see ${log_file}"
  fi

  log "${name} started with pid ${pid}"
}

cd "$SCRIPT_DIR"

if [ ! -f "$ENV_FILE" ]; then
  cp "${SCRIPT_DIR}/.env.example" "$ENV_FILE"
  log "created ${ENV_FILE}; edit it before production deployment"
fi

set -a
# shellcheck disable=SC1090
. "$ENV_FILE"
set +a

INSTALL_DIR="$(abspath_from_script_dir "${MEAT_MEMORY_INSTALL_DIR:-./bin}")"
CONFIG_DIR="$(abspath_from_script_dir "${MEAT_MEMORY_CONFIG_DIR:-./config}")"
MARKDOWN_ROOT="$(abspath_from_script_dir "${MEAT_MEMORY_MARKDOWN_ROOT:-./data/markdown}")"
ASSETS_ROOT="$(abspath_from_script_dir "${MEAT_MEMORY_ASSETS_ROOT:-./data/assets}")"

export MEAT_MEMORY_INSTALL_DIR="$INSTALL_DIR"
export MEAT_MEMORY_CONFIG_DIR="$CONFIG_DIR"
export MEAT_MEMORY_MARKDOWN_ROOT="$MARKDOWN_ROOT"
export MEAT_MEMORY_ASSETS_ROOT="$ASSETS_ROOT"

if [ -n "${MEAT_MEMORY_CONFIG:-}" ]; then
  export MEAT_MEMORY_CONFIG="$(abspath_from_script_dir "$MEAT_MEMORY_CONFIG")"
else
  export MEAT_MEMORY_CONFIG="${CONFIG_DIR}/app.toml"
fi

mkdir -p "$INSTALL_DIR" "$CONFIG_DIR" "$MARKDOWN_ROOT" "$ASSETS_ROOT" "${SCRIPT_DIR}/logs" "${SCRIPT_DIR}/run"

if [ ! -x "${INSTALL_DIR}/memory-app" ] || [ ! -x "${INSTALL_DIR}/memory-worker" ] || [ ! -x "${INSTALL_DIR}/memory-cli" ]; then
  log "binaries not found in ${INSTALL_DIR}; running install.sh"
  MEAT_MEMORY_INSTALL_DIR="$INSTALL_DIR" \
  MEAT_MEMORY_CONFIG_DIR="$CONFIG_DIR" \
  "${SCRIPT_DIR}/install.sh" --skip-tui
fi

start_process "memory-app" "${INSTALL_DIR}/memory-app"
start_process "memory-worker" "${INSTALL_DIR}/memory-worker"

log "logs: ${SCRIPT_DIR}/logs"
log "stop with: ${SCRIPT_DIR}/dev-down.sh"
log "health check: curl -fsS http://${MEAT_MEMORY_SERVER_BIND:-127.0.0.1:8080}/healthz"
