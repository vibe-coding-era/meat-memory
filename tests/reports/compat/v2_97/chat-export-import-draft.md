# V2.97-A Connector Import Draft

schema_version: 2.97-A
connector: chat-export
root_path: /Users/Rou/dev_projects/meat-memory/target/v2_97-acceptance-fixtures/chat
mode: import_draft
draft_count: 1

## Import Policy

- explicit import only; this report does not write memory records
- review is required before apply

## Drafts

- V2.97 chat import fixture [summary]
  - external_id: v297_acceptance_chat
  - source_ref: file:///Users/Rou/dev_projects/meat-memory/target/v2_97-acceptance-fixtures/chat/chat.json#v297_acceptance_chat

## Failures

- none

## Coverage Gate

- V2.97-A connector import-draft production regions require 100% targeted test coverage.
