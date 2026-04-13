#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="${1:-$ROOT_DIR/tests/reports/perf/latest}"

mkdir -p "$OUT_DIR"

echo "[v2-perf] running memory-kernel perf smoke"
cargo test -p memory-kernel --test perf_smoke -- --ignored --nocapture \
  | tee "$OUT_DIR/kernel-perf.txt"

echo "[v2-perf] running memory-sync perf smoke"
cargo test -p memory-sync --test perf_smoke -- --ignored --nocapture \
  | tee "$OUT_DIR/sync-perf.txt"

echo "[v2-perf] reports written to:"
echo "  $OUT_DIR/kernel-perf.txt"
echo "  $OUT_DIR/sync-perf.txt"
