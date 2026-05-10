#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPORT_DIR="${V2_97_ACCEPTANCE_REPORT_DIR:-${ROOT_DIR}/tests/reports/compat/v2_97}"
FIXTURE_DIR="${V2_97_ACCEPTANCE_FIXTURE_DIR:-${ROOT_DIR}/target/v2_97-acceptance-fixtures}"
ACCEPTANCE_REPORT="${REPORT_DIR}/v2_97-acceptance.txt"
COVERAGE_REPORT="${REPORT_DIR}/coverage-v2_97-new-code.md"

mkdir -p "${REPORT_DIR}" "${FIXTURE_DIR}"
rm -rf "${FIXTURE_DIR}/docs" "${FIXTURE_DIR}/chat" "${FIXTURE_DIR}/web"
mkdir -p "${FIXTURE_DIR}/docs" "${FIXTURE_DIR}/chat" "${FIXTURE_DIR}/web"
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
---
title: V2.97 Connector Fixture
tags:
  - connector
  - acceptance
summary: |
  Markdown docs connector acceptance fixture.
owner:
  team: memory
---
# V2.97 Connector Fixture

Markdown docs connector acceptance fixture.
EOF

git -C "${FIXTURE_DIR}/docs" init -b main >/dev/null
git -C "${FIXTURE_DIR}/docs" config user.name "Ada"
git -C "${FIXTURE_DIR}/docs" config user.email "ada@example.test"
git -C "${FIXTURE_DIR}/docs" remote add origin git@example.test:team/v297-fixture.git
git -C "${FIXTURE_DIR}/docs" add README.md
GIT_AUTHOR_DATE="2024-03-09T00:00:00+0000" \
GIT_COMMITTER_DATE="2024-03-09T00:00:00+0000" \
  git -C "${FIXTURE_DIR}/docs" commit -m "add V2.97 docs" >/dev/null
BASE_SHA="$(git -C "${FIXTURE_DIR}/docs" rev-parse HEAD)"
cat >"${FIXTURE_DIR}/docs/tracked.txt" <<'EOF'
tracked fixture
EOF
git -C "${FIXTURE_DIR}/docs" add tracked.txt
GIT_AUTHOR_DATE="2024-03-09T00:01:00+0000" \
GIT_COMMITTER_DATE="2024-03-09T00:01:00+0000" \
  git -C "${FIXTURE_DIR}/docs" commit -m "update V2.97 connector fixture" >/dev/null
HEAD_SHA="$(git -C "${FIXTURE_DIR}/docs" rev-parse HEAD)"
cat >"${FIXTURE_DIR}/docs/.git/packed-refs" <<EOF
# pack-refs with: peeled fully-peeled sorted
${BASE_SHA} refs/tags/v2.97
${BASE_SHA} refs/heads/main
${HEAD_SHA} refs/remotes/origin/main
EOF
cat >"${FIXTURE_DIR}/docs/scratch.tmp" <<'EOF'
untracked fixture
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

cat >"${FIXTURE_DIR}/web/allowlist.txt" <<'EOF'
example.com
EOF
cat >"${FIXTURE_DIR}/web/index.html" <<'EOF'
<!doctype html>
<html>
  <head>
    <link rel="canonical" href="https://example.com/v2.97/web-fixture" />
    <title>V2.97 Web Fixture</title>
  </head>
  <body>
    <h1>Web fixture heading</h1>
    <p>Web crawler connector acceptance fixture.</p>
    <a href="/docs">Docs</a>
  </body>
</html>
EOF

run_step "format check" cargo fmt --all --check

run_step "interface check" \
  cargo check -p memory-kernel -p memory-cli

run_step "kernel V2.97 connector contract" \
  cargo test -p memory-kernel --lib v297_ -- --test-threads=1

run_step "CLI V2.97 connector contract" \
  cargo test -p memory-cli --bin memory-cli connector_ -- --test-threads=1

run_step "CLI V2.97 confirmed connector executor contract" \
  cargo test -p memory-cli --bin memory-cli cli_command_functions_cover_pg_management_paths -- --test-threads=1

run_step "HTTP V2.97 connector management contract" \
  cargo test -p memory-http --lib v297_http_connector_ -- --test-threads=1

run_step "HTTP project document frontmatter persistence contract" \
  cargo test -p memory-http --test http_api_tests http_project_document_sync_scans_imports_and_lists_documents -- --test-threads=1

run_step "MCP V2.97 connector management contract" \
  cargo test -p memory-mcp --lib v297_mcp_connector_ -- --test-threads=1

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

echo "[v2.97] markdown-docs proposal-queue report"
"${MEMORY_CLI}" compat connector-proposal-queue \
    --connector markdown-docs \
    --root-path "${FIXTURE_DIR}/docs" \
    --scope-id scp_v297_acceptance \
    --output-dir "${REPORT_DIR}" \
    --json >"${REPORT_DIR}/markdown-docs-proposal-queue-cli.json"

echo "[v2.97] local-git sync-plan report"
"${MEMORY_CLI}" compat connector-sync-plan \
    --connector local-git \
    --root-path "${FIXTURE_DIR}/docs" \
    --scope-id scp_v297_acceptance \
    --output-dir "${REPORT_DIR}" \
    --json >"${REPORT_DIR}/local-git-sync-plan-cli.json"

echo "[v2.97] local-git proposal-queue report"
"${MEMORY_CLI}" compat connector-proposal-queue \
    --connector local-git \
    --root-path "${FIXTURE_DIR}/docs" \
    --scope-id scp_v297_acceptance \
    --output-dir "${REPORT_DIR}" \
    --json >"${REPORT_DIR}/local-git-proposal-queue-cli.json"

echo "[v2.97] chat-export dry-run report"
"${MEMORY_CLI}" compat connector-dry-run \
    --connector chat-export \
    --root-path "${FIXTURE_DIR}/chat" \
    --output-dir "${REPORT_DIR}" \
    --json >"${REPORT_DIR}/chat-export-dry-run-cli.json"

echo "[v2.97] web-crawler dry-run report"
"${MEMORY_CLI}" compat connector-dry-run \
    --connector web-crawler \
    --root-path "${FIXTURE_DIR}/web" \
    --output-dir "${REPORT_DIR}" \
    --json >"${REPORT_DIR}/web-crawler-dry-run-cli.json"

echo "[v2.97] web-crawler sync-plan report"
"${MEMORY_CLI}" compat connector-sync-plan \
    --connector web-crawler \
    --root-path "${FIXTURE_DIR}/web" \
    --scope-id scp_v297_acceptance \
    --output-dir "${REPORT_DIR}" \
    --json >"${REPORT_DIR}/web-crawler-sync-plan-cli.json"

echo "[v2.97] chat-export import-draft report"
"${MEMORY_CLI}" compat connector-import-draft \
    --connector chat-export \
    --root-path "${FIXTURE_DIR}/chat" \
    --scope-id scp_v297_acceptance \
    --output-dir "${REPORT_DIR}" \
    --proposal \
    --json >"${REPORT_DIR}/chat-export-import-draft-cli.json"

echo "[v2.97] chat-export proposal-queue report"
"${MEMORY_CLI}" compat connector-proposal-queue \
    --connector chat-export \
    --root-path "${FIXTURE_DIR}/chat" \
    --scope-id scp_v297_acceptance \
    --output-dir "${REPORT_DIR}" \
    --json >"${REPORT_DIR}/chat-export-proposal-queue-cli.json"

CHAT_QUEUE_ITEM_ID="$(python3 - <<'PY' "${REPORT_DIR}/chat-export-proposal-queue-cli.json"
import json
import sys
payload = json.load(open(sys.argv[1]))
print(payload["queue_items"][0]["queue_item_id"])
PY
)"
CHAT_CONFIRMATION_TOKEN="$(python3 - <<'PY' "${REPORT_DIR}/chat-export-proposal-queue-cli.json"
import json
import sys
payload = json.load(open(sys.argv[1]))
print(payload["queue_items"][0]["review_token"])
PY
)"

echo "[v2.97] chat-export proposal apply-plan report"
"${MEMORY_CLI}" compat connector-proposal-apply-plan \
    --connector chat-export \
    --root-path "${FIXTURE_DIR}/chat" \
    --queue-file "${REPORT_DIR}/chat-export-proposal-queue.json" \
    --scope-id scp_v297_acceptance \
    --approve-queue-item "${CHAT_QUEUE_ITEM_ID}" \
    --confirmation-token "${CHAT_CONFIRMATION_TOKEN}" \
    --output-dir "${REPORT_DIR}" \
    --json >"${REPORT_DIR}/chat-export-proposal-apply-plan-cli.json"

test -f "${REPORT_DIR}/markdown-docs-dry-run.json"
test -f "${REPORT_DIR}/markdown-docs-sync-plan.json"
test -f "${REPORT_DIR}/markdown-docs-proposal-queue.json"
test -f "${REPORT_DIR}/local-git-sync-plan.json"
test -f "${REPORT_DIR}/local-git-proposal-queue.json"
test -f "${REPORT_DIR}/chat-export-dry-run.json"
test -f "${REPORT_DIR}/web-crawler-dry-run.json"
test -f "${REPORT_DIR}/web-crawler-sync-plan.json"
test -f "${REPORT_DIR}/chat-export-import-draft.json"
test -f "${REPORT_DIR}/chat-export-proposal-queue.json"
test -f "${REPORT_DIR}/chat-export-proposal-apply-plan.json"
grep -q "Conflict Review" "${REPORT_DIR}/markdown-docs-sync-plan.md"
grep -q "Connector Proposal Queue" "${REPORT_DIR}/markdown-docs-proposal-queue.md"
grep -q "explicit import only" "${REPORT_DIR}/chat-export-import-draft.md"
grep -q "Proposal Drafts" "${REPORT_DIR}/chat-export-import-draft.md"
grep -q "V2.97 Web Fixture" "${REPORT_DIR}/web-crawler-dry-run.md"
grep -q "V2.97 Web Fixture" "${REPORT_DIR}/web-crawler-sync-plan.md"
grep -q "review queue only" "${REPORT_DIR}/chat-export-proposal-queue.md"
grep -q "Connector Proposal Apply Plan" "${REPORT_DIR}/chat-export-proposal-apply-plan.md"

python3 - <<'PY' \
  "${REPORT_DIR}/markdown-docs-dry-run-cli.json" \
  "${REPORT_DIR}/markdown-docs-sync-plan-cli.json" \
  "${REPORT_DIR}/local-git-sync-plan-cli.json" \
  "${REPORT_DIR}/chat-export-dry-run-cli.json" \
  "${REPORT_DIR}/web-crawler-dry-run-cli.json" \
  "${REPORT_DIR}/web-crawler-sync-plan-cli.json" \
  "${REPORT_DIR}/chat-export-import-draft-cli.json" \
  "${REPORT_DIR}/markdown-docs-proposal-queue-cli.json" \
  "${REPORT_DIR}/local-git-proposal-queue-cli.json" \
  "${REPORT_DIR}/chat-export-proposal-queue-cli.json" \
  "${REPORT_DIR}/chat-export-proposal-apply-plan-cli.json"
import json
import sys

markdown_dry_run = json.load(open(sys.argv[1]))
markdown_sync_plan = json.load(open(sys.argv[2]))
local_git_sync_plan = json.load(open(sys.argv[3]))
chat_dry_run = json.load(open(sys.argv[4]))
web_crawler_dry_run = json.load(open(sys.argv[5]))
web_crawler_sync_plan = json.load(open(sys.argv[6]))
chat_import_draft = json.load(open(sys.argv[7]))
markdown_proposal_queue = json.load(open(sys.argv[8]))
local_git_proposal_queue = json.load(open(sys.argv[9]))
chat_proposal_queue = json.load(open(sys.argv[10]))
chat_apply_plan = json.load(open(sys.argv[11]))

for payload in (
    markdown_dry_run,
    markdown_sync_plan,
    local_git_sync_plan,
    chat_dry_run,
    web_crawler_dry_run,
    web_crawler_sync_plan,
    chat_import_draft,
    markdown_proposal_queue,
    local_git_proposal_queue,
    chat_proposal_queue,
    chat_apply_plan,
):
    assert payload["schema_version"] == "2.97-A", payload
    assert payload["coverage_gate"]["new_feature_test_coverage_required"] == "100%", payload

assert markdown_dry_run["connector"] == "markdown-docs", markdown_dry_run
assert markdown_dry_run["candidate_count"] == 1, markdown_dry_run
assert markdown_dry_run["items"][0]["title"] == "V2.97 Connector Fixture", markdown_dry_run
assert markdown_dry_run["items"][0]["metadata"]["frontmatter_present"] is True, markdown_dry_run
assert markdown_dry_run["items"][0]["metadata"]["frontmatter"]["tags"][0] == "connector", markdown_dry_run
assert markdown_dry_run["items"][0]["metadata"]["frontmatter"]["owner"]["team"] == "memory", markdown_dry_run
assert markdown_dry_run["incremental_checkpoint"]["frontmatter"] == "parse_yaml_frontmatter_when_present", markdown_dry_run
assert markdown_sync_plan["connector"] == "markdown-docs", markdown_sync_plan
assert markdown_sync_plan["planned_count"] == 1, markdown_sync_plan
assert markdown_sync_plan["documents"][0]["metadata"]["frontmatter_present"] is True, markdown_sync_plan
assert markdown_sync_plan["documents"][0]["metadata"]["frontmatter"]["summary"].strip() == "Markdown docs connector acceptance fixture.", markdown_sync_plan
assert markdown_sync_plan["documents"][0]["metadata"]["frontmatter"]["owner"]["team"] == "memory", markdown_sync_plan
assert "frontmatter_metadata_persistence" in markdown_sync_plan["coverage_gate"]["covered_regions"], markdown_sync_plan
assert "yaml_frontmatter_compatibility" in markdown_sync_plan["coverage_gate"]["covered_regions"], markdown_sync_plan
assert "conflicts" in markdown_sync_plan, markdown_sync_plan
assert local_git_sync_plan["connector"] == "local-git", local_git_sync_plan
assert local_git_sync_plan["planned_count"] == 1, local_git_sync_plan
assert local_git_sync_plan["incremental_checkpoint"]["repository_metadata"]["git_head_ref"] == "ref: refs/heads/main", local_git_sync_plan
assert local_git_sync_plan["incremental_checkpoint"]["repository_metadata"]["active_branch"] == "main", local_git_sync_plan
repo_metadata = local_git_sync_plan["incremental_checkpoint"]["repository_metadata"]
branch_sha = repo_metadata["branches"][0]["sha"]
assert local_git_sync_plan["incremental_checkpoint"]["repository_metadata"]["branch_count"] == 1, local_git_sync_plan
assert local_git_sync_plan["incremental_checkpoint"]["repository_metadata"]["remote_count"] == 1, local_git_sync_plan
assert local_git_sync_plan["incremental_checkpoint"]["repository_metadata"]["packed_ref_count"] == 3, local_git_sync_plan
assert local_git_sync_plan["incremental_checkpoint"]["repository_metadata"]["packed_refs"][0]["kind"] == "tag", local_git_sync_plan
packed_main = [ref for ref in repo_metadata["packed_refs"] if ref["name"] == "refs/heads/main"][0]
assert packed_main["sha"] != branch_sha, local_git_sync_plan
assert local_git_sync_plan["incremental_checkpoint"]["repository_metadata"]["ref_count"] == 3, local_git_sync_plan
assert local_git_sync_plan["incremental_checkpoint"]["repository_metadata"]["refs"][0]["name"] == "refs/heads/main", local_git_sync_plan
assert local_git_sync_plan["incremental_checkpoint"]["repository_metadata"]["refs"][0]["sha"] == branch_sha, local_git_sync_plan
assert local_git_sync_plan["incremental_checkpoint"]["repository_metadata"]["refs"][0]["source"] == "loose", local_git_sync_plan
assert local_git_sync_plan["incremental_checkpoint"]["repository_metadata"]["refs"][1]["name"] == "refs/remotes/origin/main", local_git_sync_plan
assert local_git_sync_plan["incremental_checkpoint"]["repository_metadata"]["refs"][2]["name"] == "refs/tags/v2.97", local_git_sync_plan
assert local_git_sync_plan["incremental_checkpoint"]["repository_metadata"]["worktree_status"]["git_index_present"] is True, local_git_sync_plan
assert local_git_sync_plan["incremental_checkpoint"]["repository_metadata"]["worktree_status"]["status_available"] is True, local_git_sync_plan
assert local_git_sync_plan["incremental_checkpoint"]["repository_metadata"]["worktree_status"]["status_source"] == "git_status_porcelain_v1", local_git_sync_plan
assert local_git_sync_plan["incremental_checkpoint"]["repository_metadata"]["worktree_status"]["dirty_state"] == "dirty", local_git_sync_plan
assert local_git_sync_plan["incremental_checkpoint"]["repository_metadata"]["worktree_status"]["status_summary"]["untracked_count"] == 1, local_git_sync_plan
assert local_git_sync_plan["incremental_checkpoint"]["repository_metadata"]["worktree_status"]["status_entries"][0]["path"] == "scratch.tmp", local_git_sync_plan
assert local_git_sync_plan["incremental_checkpoint"]["repository_metadata"]["commit_count"] == 2, local_git_sync_plan
assert local_git_sync_plan["incremental_checkpoint"]["repository_metadata"]["recent_commits"][0]["sha"] == branch_sha, local_git_sync_plan
assert local_git_sync_plan["incremental_checkpoint"]["repository_metadata"]["important_files"][0]["relative_path"] == "README.md", local_git_sync_plan
assert chat_dry_run["connector"] == "chat-export", chat_dry_run
assert chat_dry_run["items"][0]["metadata"]["message_count"] == 2, chat_dry_run
assert web_crawler_dry_run["connector"] == "web-crawler", web_crawler_dry_run
assert web_crawler_dry_run["candidate_count"] == 1, web_crawler_dry_run
assert web_crawler_dry_run["items"][0]["title"] == "V2.97 Web Fixture", web_crawler_dry_run
assert web_crawler_dry_run["items"][0]["metadata"]["canonical_url"] == "https://example.com/v2.97/web-fixture", web_crawler_dry_run
assert web_crawler_dry_run["items"][0]["metadata"]["allowlist_allowed"] is True, web_crawler_dry_run
assert web_crawler_dry_run["items"][0]["metadata"]["allowlist_domains"][0] == "example.com", web_crawler_dry_run
assert web_crawler_dry_run["items"][0]["metadata"]["link_count"] == 1, web_crawler_dry_run
assert web_crawler_dry_run["items"][0]["metadata"]["remote_network"] is False, web_crawler_dry_run
assert web_crawler_dry_run["items"][0]["metadata"]["visible_text_bytes"] > 0, web_crawler_dry_run
assert "web_crawler_dry_run" in web_crawler_dry_run["coverage_gate"]["covered_regions"], web_crawler_dry_run
assert web_crawler_sync_plan["connector"] == "web-crawler", web_crawler_sync_plan
assert web_crawler_sync_plan["planned_count"] == 1, web_crawler_sync_plan
assert web_crawler_sync_plan["documents"][0]["title"] == "V2.97 Web Fixture", web_crawler_sync_plan
assert web_crawler_sync_plan["documents"][0]["canonical_uri"] == "https://example.com/v2.97/web-fixture", web_crawler_sync_plan
assert web_crawler_sync_plan["documents"][0]["metadata"]["allowlist_allowed"] is True, web_crawler_sync_plan
assert web_crawler_sync_plan["documents"][0]["metadata"]["fetch_policy"] == "local_snapshot_only_no_remote_fetch", web_crawler_sync_plan
assert web_crawler_sync_plan["documents"][0]["metadata"]["link_count"] == 1, web_crawler_sync_plan
assert web_crawler_sync_plan["incremental_checkpoint"]["blocked_count"] == 0, web_crawler_sync_plan
assert web_crawler_sync_plan["incremental_checkpoint"]["remote_network"] is False, web_crawler_sync_plan
assert "web_crawler_sync_plan" in web_crawler_sync_plan["coverage_gate"]["covered_regions"], web_crawler_sync_plan
assert chat_import_draft["connector"] == "chat-export", chat_import_draft
assert chat_import_draft["draft_count"] == 1, chat_import_draft
assert chat_import_draft["proposal_draft_count"] == 1, chat_import_draft
assert chat_import_draft["proposal_drafts"][0]["proposal_type"] == "distill_upsert", chat_import_draft
assert chat_import_draft["proposal_drafts"][0]["review_level"] == "required", chat_import_draft
assert chat_import_draft["import_policy"]["writes_memory"] is False, chat_import_draft
assert chat_import_draft["import_policy"]["proposal_mode"] is True, chat_import_draft
assert chat_import_draft["drafts"][0]["source_refs"][0].endswith("chat.json#v297_acceptance_chat"), chat_import_draft
assert markdown_proposal_queue["connector"] == "markdown-docs", markdown_proposal_queue
assert markdown_proposal_queue["mode"] == "proposal_queue", markdown_proposal_queue
assert markdown_proposal_queue["queue_item_count"] == 1, markdown_proposal_queue
assert markdown_proposal_queue["queue_items"][0]["proposal_type"] == "project_document_upsert", markdown_proposal_queue
assert markdown_proposal_queue["queue_items"][0]["review_token"].startswith("confirm_"), markdown_proposal_queue
assert markdown_proposal_queue["queue_policy"]["writes_project_documents"] is False, markdown_proposal_queue
assert local_git_proposal_queue["connector"] == "local-git", local_git_proposal_queue
assert local_git_proposal_queue["queue_item_count"] == 1, local_git_proposal_queue
assert local_git_proposal_queue["incremental_checkpoint"]["repository_metadata"]["worktree_status"]["dirty_state"] == "dirty", local_git_proposal_queue
assert chat_proposal_queue["connector"] == "chat-export", chat_proposal_queue
assert chat_proposal_queue["mode"] == "proposal_queue", chat_proposal_queue
assert chat_proposal_queue["queue_item_count"] == 1, chat_proposal_queue
assert chat_proposal_queue["queue_items"][0]["proposal_type"] == "distill_upsert", chat_proposal_queue
assert chat_proposal_queue["queue_items"][0]["review_level"] == "required", chat_proposal_queue
assert chat_proposal_queue["queue_items"][0]["review_token"].startswith("confirm_"), chat_proposal_queue
assert chat_proposal_queue["queue_policy"]["writes_memory"] is False, chat_proposal_queue
assert chat_apply_plan["connector"] == "chat-export", chat_apply_plan
assert chat_apply_plan["mode"] == "proposal_apply_plan", chat_apply_plan
assert chat_apply_plan["selected_count"] == 1, chat_apply_plan
assert chat_apply_plan["applicable_count"] == 1, chat_apply_plan
assert chat_apply_plan["apply_items"][0]["queue_item_id"] == chat_proposal_queue["queue_items"][0]["queue_item_id"], chat_apply_plan
assert chat_apply_plan["apply_policy"]["writes_memory"] is False, chat_apply_plan
assert chat_apply_plan["apply_policy"]["writes_project_documents"] is False, chat_apply_plan
assert chat_apply_plan["apply_policy"]["requires_confirmation_token"] is True, chat_apply_plan
assert chat_apply_plan["apply_policy"]["executor_not_invoked"] is True, chat_apply_plan
assert "connector_persistent_queue_manifest" in chat_apply_plan["coverage_gate"]["covered_regions"], chat_apply_plan
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
- markdown-docs frontmatter metadata persistence
- markdown-docs full YAML frontmatter compatibility
- markdown-docs explicit apply path
- source auto discovery for connector apply
- chat-export JSON parser
- chat-export import-draft projection
- chat-export explicit apply path
- web-crawler dry-run / canonical URL / allowlist
- web-crawler sync-plan / project document projection
- connector proposal-queue projection
- connector proposal apply-plan projection
- connector queue confirmation token
- connector persistent queue manifest
- service-side proposal store
- connector proposal confirmed executor
- service-side confirmed executor
- markdown-docs proposal queue
- local-git proposal queue
- chat-export proposal queue
- chat-export proposal apply-plan
- CLI parser and command projections
- CLI connector queue-file manifest
- CLI confirmed connector executor
- HTTP connector dry-run endpoint
- HTTP connector sync-plan endpoint
- HTTP connector import-draft endpoint
- HTTP connector proposal-queue endpoint
- HTTP connector persisted queue endpoint
- HTTP connector proposal apply-plan endpoint
- HTTP connector confirmed executor endpoint
- Browser Console connector confirmed executor UI
- Browser Console connector proposal store UI
- MCP connector dry-run tool
- MCP connector sync-plan tool
- MCP connector import-draft tool
- MCP connector proposal-queue tool
- MCP connector proposal apply-plan tool
- MCP connector confirmed executor tool

Evidence:

- `cargo test -p memory-kernel --lib v297_`
- `cargo test -p memory-cli --bin memory-cli connector_`
- `cargo test -p memory-http --lib v297_http_connector_`
- `cargo test -p memory-mcp --lib v297_mcp_connector_`
- `cargo test -p memory-cli --bin memory-cli cli_command_functions_cover_pg_management_paths`
- `docs/scripts/v2_97-acceptance.sh`
EOF

grep -q "Status: passed" "${COVERAGE_REPORT}"
grep -q "100% targeted coverage" "${COVERAGE_REPORT}"

echo "[v2.97] acceptance passed"
