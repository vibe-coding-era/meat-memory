#!/usr/bin/env bash
set -euo pipefail

check_cmd() {
  local name="$1"
  local cmd="$2"

  if eval "${cmd}" >/dev/null 2>&1; then
    echo "PASS ${name}"
  else
    echo "FAIL ${name}"
    return 1
  fi
}

status=0
PGVECTOR_URL="${MEAT_MEMORY_PGVECTOR_URL:-postgres://postgres:postgres@127.0.0.1:5433/meat_memory_dev}"

check_cmd "xcode-clt" "xcode-select -p" || status=1
check_cmd "clang" "clang --version" || status=1
check_cmd "homebrew" "brew --version" || status=1
check_cmd "rustc" "rustc --version" || status=1
check_cmd "cargo" "cargo --version" || status=1
check_cmd "rustfmt" "cargo fmt --version" || status=1
check_cmd "clippy" "cargo clippy -V" || status=1
check_cmd "psql" "psql --version" || status=1
check_cmd "pg_isready" "pg_isready" || status=1
check_cmd "pgvector-target" "psql \"${PGVECTOR_URL}\" -Atqc \"select extname from pg_extension where extname = 'vector';\" | grep -qx vector" || status=1
check_cmd "docker" "docker version" || status=1
check_cmd "docker-compose" "docker compose version" || status=1
check_cmd "kubectl" "kubectl version --client" || status=1
check_cmd "helm" "helm version --short" || status=1
check_cmd "terraform" "terraform version" || status=1
check_cmd "ffmpeg" "ffmpeg -version" || status=1
check_cmd "openssl" "openssl version" || status=1
check_cmd "pkg-config" "pkg-config --version" || status=1

exit "${status}"
