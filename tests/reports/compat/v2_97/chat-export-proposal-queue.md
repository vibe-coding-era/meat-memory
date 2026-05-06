# V2.97-A Connector Proposal Queue

schema_version: 2.97-A
connector: chat-export
root_path: /Users/Rou/dev_projects/meat-memory/target/v2_97-acceptance-fixtures/chat
mode: proposal_queue
queue_item_count: 1
blocked_count: 0

## Queue Policy

- review queue only; this report does not write memory or project documents
- review is required before apply
- service apply is not exposed by this compatibility endpoint

## Queue Items

- V2.97 chat import fixture [distill_upsert / required]
  - id: cpq_chat-export_0_v297_acceptance_chat
  - apply_target: Kernel::remember_text_after_review
  - blocked: false

## Failures

- none

## Coverage Gate

- V2.97-A connector proposal-queue production regions require 100% targeted test coverage.
