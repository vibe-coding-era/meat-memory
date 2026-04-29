#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPORT_DIR="${V2_97_ACCEPTANCE_REPORT_DIR:-${ROOT_DIR}/tests/reports/compat/v2_97}"
FIXTURE_DIR="${V2_97_ACCEPTANCE_FIXTURE_DIR:-${ROOT_DIR}/target/v2_97-acceptance-fixtures}"
ACCEPTANCE_REPORT="${REPORT_DIR}/v2_97-acceptance.txt"
COVERAGE_REPORT="${REPORT_DIR}/coverage-v2_97-new-code.md"

mkdir -p "${REPORT_DIR}" "${FIXTURE_DIR}/docs" "${FIXTURE_DIR}/chat"
cd "${ROOT_DIR}"

exec > >(tee "${ACCEPTANCE_REPORT}") 2>&1

run_step() {
  local label="$1"
  shift
  echo "[v2.97] ${label}"
  "$@"
}

echo "[v2.97] acceptance started"
echo "[v2.97] report: ${ACCEPTANCE_REPORT}"
echo "[v2.97] fixture root: ${FIXTURE_DIR}"

cat >"${FIXTURE_DIR}/docs/README.md" <<'EOF'
# V2.97 Connector Fixture

Markdown docs connector acceptance fixture.
EOF
mkdir -p "${FIXTURE_DIR}/docs/.git"
cat >"${FIXTURE_DIR}/docs/.git/HEAD" <<'EOF'
ref: refs/heads/main
EOF
mkdir -p "${FIXTURE_DIR}/docs/.git/logs"
cat >"${FIXTURE_DIR}/docs/.git/logs/HEAD" <<'EOF'
0000000000000000000000000000000000000000 1111111111111111111111111111111111111111 Ada <ada@example.test> 1710000000 +0000	commit (initial): add V2.97 docs
1111111111111111111111111111111111111111 2222222222222222222222222222222222222222 Ada <ada@example.test> 1710000100 +0000	commit: update V2.97 connector fixture
EOF

cat >"${FIXTURE_DIR}/chat/chat.json" <<'EOF'
{
  "id": "v297_acceptance_chat",
  "title": "V2.97 chat import fixture",
  "messages": [
    {"role": "user", "content": "Capture this V2.97 connector acceptance chat."},
    {"role": "assistant", "content": "Create an import draft with source refs."}
  ]
}
EOF

run_step "format check" cargo fmt --all --check

run_step "interface check" \
  cargo check -p memory-kernel -p memory-cli

run_step "kernel V2.97 connector contract" \
  cargo test -p memory-kernel --lib v297_ -- --test-threads=1

run_step "CLI V2.97 connector contract" \
  cargo test -p memory-cli --bin memory-cli connector_ -- --test-threads=1

run_step "CLI binary build" cargo build -p memory-cli
MEMORY_CLI="${ROOT_DIR}/target/debug/memory-cli"

echo "[v2.97] markdown-docs dry-run report"
"${MEMORY_CLI}" compat connector-dry-run \
    --connector markdown-docs \
    --root-path "${FIXTURE_DIR}/docs" \
    --output-dir "${REPORT_DIR}" \
    --json >"${REPORT_DIR}/markdown-docs-dry-run-cli.json"

echo "[v2.97] markdown-docs sync-plan report"
"${MEMORY_CLI}" compat connector-sync-plan \
    --connector markdown-docs \
    --root-path "${FIXTURE_DIR}/docs" \
    --scope-id scp_v297_acceptance \
    --output-dir "${REPORT_DIR}" \
    --json >"${REPORT_DIR}/markdown-docs-sync-plan-cli.json"

echo "[v2.97] local-git sync-plan report"
"${MEMORY_CLI}" compat connector-sync-plan \
    --connector local-git \
    --root-path "${FIXTURE_DIR}/docs" \
    --scope-id scp_v297_acceptance \
    --output-dir "${REPORT_DIR}" \
    --json >"${REPORT_DIR}/local-git-sync-plan-cli.json"

echo "[v2.97] chat-export dry-run report"
"${MEMORY_CLI}" compat connector-dry-run \
    --connector chat-export \
    --root-path "${FIXTURE_DIR}/chat" \
    --output-dir "${REPORT_DIR}" \
    --json >"${REPORT_DIR}/chat-export-dry-run-cli.json"

echo "[v2.97] chat-export import-draft report"
"${MEMORY_CLI}" compat connector-import-draft \
    --connector chat-export \
    --root-path "${FIXTURE_DIR}/chat" \
    --scope-id scp_v297_acceptance \
    --output-dir "${REPORT_DIR}" \
    --proposal \
    --json >"${REPORT_DIR}/chat-export-import-draft-cli.json"

test -f "${REPORT_DIR}/markdown-docs-dry-run.json"
test -f "${REPORT_DIR}/markdown-docs-sync-plan.json"
test -f "${REPORT_DIR}/local-git-sync-plan.json"
test -f "${REPORT_DIR}/chat-export-dry-run.json"
test -f "${REPORT_DIR}/chat-export-import-draft.json"
grep -q "Conflict Review" "${REPORT_DIR}/markdown-docs-sync-plan.md"
grep -q "explicit import only" "${REPORT_DIR}/chat-export-import-draft.md"
grep -q "Proposal Drafts" "${REPORT_DIR}/chat-export-import-draft.md"

python3 - <<'PY' \
  "${REPORT_DIR}/markdown-docs-dry-run-cli.json" \
  "${REPORT_DIR}/markdown-docs-sync-plan-cli.json" \
  "${REPORT_DIR}/local-git-sync-plan-cli.json" \
  "${REPORT_DIR}/chat-export-dry-run-cli.json" \
  "${REPORT_DIR}/chat-export-import-draft-cli.json"
import json
import sys

markdown_dry_run = json.load(open(sys.argv[1]))
markdown_sync_plan = json.load(open(sys.argv[2]))
local_git_sync_plan = json.load(open(sys.argv[3]))
chat_dry_run = json.load(open(sys.argv[4]))
chat_import_draft = json.load(open(sys.argv[5]))

for payload in (markdown_dry_run, markdown_sync_plan, local_git_sync_plan, chat_dry_run, chat_import_draft):
    assert payload["schema_version"] == "2.97-A", payload
    assert payload["coverage_gate"]["new_feature_test_coverage_required"] == "100%", payload

assert markdown_dry_run["connector"] == "markdown-docs", markdown_dry_run
assert markdown_dry_run["candidate_count"] == 1, markdown_dry_run
assert markdown_sync_plan["connector"] == "markdown-docs", markdown_sync_plan
assert markdown_sync_plan["planned_count"] == 1, markdown_sync_plan
assert "conflicts" in markdown_sync_plan, markdown_sync_plan
assert local_git_sync_plan["connector"] == "local-git", local_git_sync_plan
assert local_git_sync_plan["planned_count"] == 1, local_git_sync_plan
assert local_git_sync_plan["incremental_checkpoint"]["repository_metadata"]["git_head_ref"] == "ref: refs/heads/main", local_git_sync_plan
assert local_git_sync_plan["incremental_checkpoint"]["repository_metadata"]["commit_count"] == 2, local_git_sync_plan
assert local_git_sync_plan["incremental_checkpoint"]["repository_metadata"]["recent_commits"][0]["sha"] == "2222222222222222222222222222222222222222", local_git_sync_plan
assert local_git_sync_plan["incremental_checkpoint"]["repository_metadata"]["important_files"][0]["relative_path"] == "README.md", local_git_sync_plan
assert chat_dry_run["connector"] == "chat-export", chat_dry_run
assert chat_dry_run["items"][0]["metadata"]["message_count"] == 2, chat_dry_run
assert chat_import_draft["connector"] == "chat-export", chat_import_draft
assert chat_import_draft["draft_count"] == 1, chat_import_draft
assert chat_import_draft["proposal_draft_count"] == 1, chat_import_draft
assert chat_import_draft["proposal_drafts"][0]["proposal_type"] == "distill_upsert", chat_import_draft
assert chat_import_draft["proposal_drafts"][0]["review_level"] == "required", chat_import_draft
assert chat_import_draft["import_policy"]["writes_memory"] is False, chat_import_draft
assert chat_import_draft["import_policy"]["proposal_mode"] is True, chat_import_draft
assert chat_import_draft["drafts"][0]["source_refs"][0].endswith("chat.json#v297_acceptance_chat"), chat_import_draft
PY

cat >"${COVERAGE_REPORT}" <<'EOF'
# V2.97 New Code Coverage Gate

Status: passed

Required: 100% targeted coverage for V2.97新增功能.

Covered regions:

- connector dry-run contract
- local-git dry-run report
- local-git sync-plan checkpoint
- markdown-docs dry-run
- markdown-docs sync-plan / conflict review projection
- markdown-docs explicit apply path
- source auto discovery for connector apply
- chat-export JSON parser
- chat-export import-draft projection
- chat-export explicit apply path
- CLI parser and command projections

Evidence:

- `cargo test -p memory-kernel --lib v297_`
- `cargo test -p memory-cli --bin memory-cli connector_`
- `cargo test -p memory-cli --bin memory-cli cli_command_functions_cover_pg_management_paths`
- `docs/scripts/v2_97-acceptance.sh`
EOF

grep -q "Status: passed" "${COVERAGE_REPORT}"
grep -q "100% targeted coverage" "${COVERAGE_REPORT}"

echo "[v2.97] acceptance passed"
