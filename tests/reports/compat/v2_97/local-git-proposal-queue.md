# V2.97-A Connector Proposal Queue

schema_version: 2.97-A
queue_id: cpq_local-git_3bbbd068e460af9e
connector: local-git
root_path: /Users/Rou/dev_projects/meat-memory/target/v2_97-acceptance-fixtures/docs
mode: proposal_queue
queue_item_count: 1
blocked_count: 0

## Queue Policy

- review queue only; this report does not write memory or project documents
- review is required before apply
- service apply is not exposed by this compatibility endpoint

## Queue Items

- V2.97 Connector Fixture [project_document_upsert / suggested]
  - id: cpq_local-git_0_file____users_rou_dev_projects_meat_memory_targe
  - review_token: confirm_7b2fc398ab52721b
  - apply_target: Kernel::apply_project_document_sync_plan
  - blocked: false

## Failures

- none

## Coverage Gate

- V2.97-A connector proposal-queue production regions require 100% targeted test coverage.
