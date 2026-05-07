# V2.97-A Connector Proposal Apply Plan

schema_version: 2.97-A
queue_id: cpq_chat-export_57892281b5421cbb
connector: chat-export
root_path: /Users/Rou/dev_projects/meat-memory/target/v2_97-acceptance-fixtures/chat
mode: proposal_apply_plan
selected_count: 1
applicable_count: 1
blocked_count: 0

## Apply Policy

- plan only; this report does not write memory or project documents
- confirmation token was verified before generating this plan
- runtime key and executor are still required for actual apply

## Apply Items

- V2.97 chat import fixture [distill_upsert]
  - id: cpq_chat-export_0_v297_acceptance_chat
  - apply_target: Kernel::remember_text_after_review
  - can_apply: true

## Skipped Items

- none

## Coverage Gate

- V2.97-A connector proposal apply-plan production regions require 100% targeted test coverage.
