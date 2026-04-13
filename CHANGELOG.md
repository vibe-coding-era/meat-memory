# Changelog

All notable changes to this project will be documented in this file.

The format is inspired by Keep a Changelog and the repository will follow semantic versioning once release automation is enabled.

## [Unreleased]

### Added

- 暂无

## [v2.5] - 2026-04-13

### Added

- Added V2.4 short-term and mid-term memory delivery scope: Agent Context, Project Documents, source multi-key support, local docs sync, conflict reporting, and Markdown project document projection.
- Added install, package, and deploy delivery closure for binary release artifacts, Agent skill bundle, cargo install, npm wrapper skeleton, Homebrew formula template, Docker app/worker images, systemd deployment, Helm deployment, and user-facing install runbook.
- Added release pipeline skeleton for GitHub Release assets, agent skills bundle, and GHCR app/worker image publishing.

### Changed

- Standardized scripts under `docs/scripts/`.
- Standardized Docker image naming as `meat-memory-app:<tag>` and `meat-memory-worker:<tag>`.
- Updated runbook, architecture, tasklist, and packaging documentation around V2.5 distribution and deployment.

### Notes

- npm and Homebrew are documented as publish-ready skeletons, not as live public channels, until final release URL, npm scope, and tap address are confirmed.

## [0.1.0] - 2026-04-09

### Added

- Initial Rust workspace scaffold
- Environment verification and bootstrap scripts
- CI workflow skeletons
- Core project documentation and task tracking artifacts
- PostgreSQL and Markdown dual-store memory pipeline
- Kernel remember/search/publish orchestration with knowledge graph extraction
- HTTP, CLI, and MCP access layers for agent integration
- Structured logging, metrics snapshots, health endpoints, and sync oplog baseline
- Provider catalog and route registry for Gemini / Claude / ChatGPT / 千问 / 豆包 / MiniMax / GLM
- LLM failover routing with runtime `llm_notice`
- Image asset storage, `remember_image`, local vision caption derivation, and image API/CLI flow
- Browser Console root route at `/`
- V1 Chinese acceptance corpus and regression coverage across extract / kernel / HTTP / MCP / CLI
- Docker Compose local stack, Dockerfile, Helm chart, and V1 deployment/acceptance runbooks
- V1 release notes and final delivery boundary documentation
