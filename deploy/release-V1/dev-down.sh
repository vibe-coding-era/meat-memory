#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

log() {
  printf '[meat-memory] %s\n' "$*"
}

stop_process() {
  local name="$1"
  local pid_file="${SCRIPT_DIR}/run/${name}.pid"

  if [ ! -f "$pid_file" ]; then
    log "${name} is not running"
    return
  fi

  local pid
  pid="$(cat "$pid_file")"
  if [ -n "$pid" ] && kill -0 "$pid" >/dev/null 2>&1; then
    log "stopping ${name} pid ${pid}"
    kill "$pid"
  else
    log "${name} pid file is stale"
  fi

  rm -f "$pid_file"
}

stop_process "memory-worker"
stop_process "memory-app"
