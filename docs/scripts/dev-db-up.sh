#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

cd "${ROOT_DIR}"
docker compose up -d pgvector

until docker compose exec -T pgvector pg_isready -U postgres -d postgres >/dev/null 2>&1; do
  sleep 1
done

psql "postgres://postgres:postgres@127.0.0.1:5433/meat_memory_dev" -Atqc \
  "select extname from pg_extension where extname = 'vector';"
