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

capture_env_overrides() {
  for name in "$@"; do
    eval "if [ \"\${${name}+set}\" = set ]; then __HAS_${name}=1; __VAL_${name}=\"\${${name}}\"; else __HAS_${name}=0; fi"
  done
}

restore_env_overrides() {
  for name in "$@"; do
    eval "if [ \"\${__HAS_${name}:-0}\" = 1 ]; then export ${name}=\"\${__VAL_${name}}\"; fi; unset __HAS_${name} __VAL_${name}"
  done
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

database_host_port() {
  local url="${MEAT_MEMORY_DATABASE_URL:-}"
  [ -n "$url" ] || return 1

  local rest="${url#*://}"
  local authority="${rest%%/*}"
  authority="${authority##*@}"

  local host="${authority%%:*}"
  local port="${authority##*:}"

  if [ "$host" = "$authority" ]; then
    port="5432"
  fi

  [ -n "$host" ] || return 1
  printf '%s %s\n' "$host" "$port"
}

is_truthy() {
  case "${1:-}" in
    1|true|TRUE|yes|YES|on|ON)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

config_feature_value() {
  local key="$1"
  local config_path="${MEAT_MEMORY_CONFIG:-}"

  [ -n "$config_path" ] && [ -f "$config_path" ] || return 1

  awk -v key="$key" '
    /^\[[^]]+\]$/ { section = $0 }
    section == "[features]" && $0 ~ "^[[:space:]]*" key "[[:space:]]*=" {
      value = $0
      sub(/^[^=]*=[[:space:]]*/, "", value)
      sub(/[[:space:]]*(#.*)?$/, "", value)
      gsub(/"/, "", value)
      print value
      exit
    }
  ' "$config_path"
}

postgres_enabled() {
  if [ "${MEAT_MEMORY_ENABLE_PG+x}" = "x" ]; then
    is_truthy "$MEAT_MEMORY_ENABLE_PG"
    return
  fi

  local config_value
  config_value="$(config_feature_value enable_pg || true)"
  [ -n "$config_value" ] && is_truthy "$config_value"
}

check_database_endpoint() {
  if [ "${MEAT_MEMORY_SKIP_DB_CHECK:-0}" = "1" ]; then
    log "skipped database preflight because MEAT_MEMORY_SKIP_DB_CHECK=1"
    return
  fi

  if ! postgres_enabled; then
    log "skipped database preflight because PostgreSQL is disabled"
    return
  fi

  local host_port
  if ! host_port="$(database_host_port)"; then
    fail "MEAT_MEMORY_DATABASE_URL is empty or unsupported; update .env before running dev-up.sh"
  fi

  local host="${host_port% *}"
  local port="${host_port#* }"

  if ! command -v nc >/dev/null 2>&1; then
    fail "nc is required for database TCP preflight; install netcat or set MEAT_MEMORY_SKIP_DB_CHECK=1 for script-only smoke tests"
  fi

  if ! nc -z "$host" "$port" >/dev/null 2>&1; then
    fail "PostgreSQL/pgvector is not reachable at ${host}:${port}; start the database, update MEAT_MEMORY_DATABASE_URL, or set MEAT_MEMORY_SKIP_DB_CHECK=1 for script-only smoke tests"
  fi

  log "database TCP preflight passed: ${host}:${port}"
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

ENV_OVERRIDE_NAMES=(
  MEAT_MEMORY_INSTALL_DIR
  MEAT_MEMORY_CONFIG_DIR
  MEAT_MEMORY_TUI_CONFIG
  MEAT_MEMORY_RUN_TUI
  MEAT_MEMORY_INSTALL_MISSING_DEPS
  MEAT_MEMORY_CONFIG
  MEAT_MEMORY_SERVER_BIND
  MEAT_MEMORY_MARKDOWN_ROOT
  MEAT_MEMORY_ASSETS_ROOT
  MEAT_MEMORY_DATABASE_URL
  MEAT_MEMORY_SKIP_DB_CHECK
  MEAT_MEMORY_ENABLE_HTTP
  MEAT_MEMORY_ENABLE_MCP
  RUST_LOG
)

if [ ! -f "$ENV_FILE" ]; then
  cp "${SCRIPT_DIR}/.env.example" "$ENV_FILE"
  log "created ${ENV_FILE}; edit it before production deployment"
fi

capture_env_overrides "${ENV_OVERRIDE_NAMES[@]}"
set -a
# shellcheck disable=SC1090
. "$ENV_FILE"
set +a
restore_env_overrides "${ENV_OVERRIDE_NAMES[@]}"

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
check_database_endpoint

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
