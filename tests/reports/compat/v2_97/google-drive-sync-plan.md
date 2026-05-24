# V2.97-A Connector Sync Plan

schema_version: 2.97-A
connector: google-drive
root_path: /Users/Rou/dev_projects/meat-memory/target/v2_97-acceptance-fixtures/drive
mode: sync_plan
planned_count: 2
missing_count: 0
conflict_count: 0

## Planned Documents

- planning (changed)
  - gdrive-export://planning
- sheet (changed)
  - gdrive-export://sheet

## Conflict Review

- none

## Evidence Preview

- gdrive-export://planning [0..33]
  - Drive PDF sidecar extracted text.
- gdrive-export://sheet [0..11]
  - name,status

## Coverage Gate

- V2.97-A connector sync-plan production regions require 100% targeted test coverage.
