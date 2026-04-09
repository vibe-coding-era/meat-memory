# Test Coverage Report

## Overview

| Item | Value |
|---|---|
| Report Date | 2026-04-09 |
| Repository | `/Users/Rou/dev_projects/meat-memory` |
| Scope | Unit tests only, production code only (`crates/*/src/**`) |
| Coverage Method | `rustc` source-based coverage (`-Cinstrument-coverage`) |
| Coverage Tools | Rust toolchain bundled `llvm-profdata` / `llvm-cov` |
| Latest Unit Build | `cargo test --workspace --lib --bins --no-run --message-format=json` |
| Latest Unit Execution | Direct execution of instrumented test binaries from `executables.txt` with absolute `LLVM_PROFILE_FILE` |
| Environment Note | Sandbox still blocks PG-backed integration and socket-bind happy-path coverage |

## Coverage Snapshot

| Metric | Latest Unit | Original Unit Baseline | Delta |
|---|---:|---:|---:|
| Line Coverage | 95.46% | 62.03% | +33.43 pp |
| Function Coverage | 91.79% | 56.65% | +35.14 pp |
| Region Coverage | 87.91% | 43.00% | +44.91 pp |

## Raw Counts

| Metric | Covered | Total |
|---|---:|---:|
| Lines | 10353 | 10845 |
| Functions | 716 | 780 |
| Regions | 3308 | 3763 |

## Recent Gains

| File | Previous Unit Line Coverage | Latest Unit Line Coverage | Change |
|---|---:|---:|---:|
| `crates/memory-extract/src/entity.rs` | 77.18% | 99.45% | +22.27 pp |
| `crates/memory-kernel/src/lib.rs` | 87.50% | 95.40% | +7.90 pp |
| `crates/memory-domain/src/artifact.rs` | 95.08% | 100.00% | +4.92 pp |
| `crates/memory-policy/src/lib.rs` | 92.86% | 96.43% | +3.57 pp |
| `crates/memory-extract/src/lib.rs` | 93.98% | 95.18% | +1.20 pp |

## Focus Files

| File | Line | Function | Region |
|---|---:|---:|---:|
| `crates/memory-http/src/lib.rs` | 100.00% | 100.00% | 98.21% |
| `crates/memory-cli/src/main.rs` | 92.31% | 73.73% | 79.04% |
| `crates/memory-worker/src/main.rs` | 90.36% | 78.95% | 64.29% |
| `crates/memory-app/src/main.rs` | 90.73% | 75.00% | 60.82% |
| `crates/memory-kernel/src/lib.rs` | 95.40% | 92.68% | 84.25% |
| `crates/memory-extract/src/entity.rs` | 99.45% | 100.00% | 98.16% |
| `crates/memory-store-pg/src/lib.rs` | 87.00% | 98.48% | 82.63% |
| `crates/memory-models/src/lib.rs` | 98.22% | 98.63% | 94.72% |
| `crates/memory-observability/src/lib.rs` | 99.22% | 92.59% | 96.67% |
| `crates/memory-config/src/lib.rs` | 99.56% | 93.75% | 97.87% |

## Current Hotspots

| Priority | File | Line | Function | Region | Note |
|---|---|---:|---:|---:|---|
| P1 | `crates/memory-domain/src/memory.rs` | 85.06% | 100.00% | 75.76% | Memory 状态迁移仍有分支缺口，是当前最低覆盖的生产文件 |
| P1 | `crates/memory-store-pg/src/lib.rs` | 87.00% | 98.48% | 82.63% | PG 数据路径与错误分支仍有剩余空白 |
| P1 | `crates/memory-assets/src/lib.rs` | 89.12% | 93.10% | 70.59% | 资产 metadata 与存储辅助分支还有补测空间 |
| P1 | `crates/memory-worker/src/main.rs` | 90.36% | 78.95% | 64.29% | 启动、主循环和退出路径仍偏薄弱 |
| P1 | `crates/memory-app/src/main.rs` | 90.73% | 75.00% | 60.82% | App bootstrap 与 serve 生命周期仍需更细粒度单测 |
| P1 | `crates/memory-cli/src/main.rs` | 92.31% | 73.73% | 79.04% | 入口函数较大，函数覆盖率仍明显低于行覆盖率 |
| P2 | `crates/memory-store-md/src/repo/memory_file_repo.rs` | 92.99% | 85.71% | 85.50% | Markdown repo 的更新与异常路径还有余量 |
| P2 | `crates/memory-mcp/src/lib.rs` | 94.86% | 90.41% | 93.19% | MCP dispatcher 已较稳，但仍有少量工具分支未触达 |

## Test Execution Status

| Command | Status | Note |
|---|---|---|
| `cargo test -p memory-kernel --lib --quiet` | Passed | P0 修复后重新通过，`serde_json` 测试依赖缺口已解除 |
| `cargo test --workspace --lib --bins --quiet` | Passed | 2026-04-09 重新回归通过，工作区恢复全绿 |
| `cargo test -p memory-http --lib` | Passed | 路由 payload、错误映射、解析函数分支已补齐 |
| `cargo test -p memory-cli --bin memory-cli` | Passed | runtime helper、stdin 分支、remember/search/image 运行路径已补齐 |
| `cargo test -p memory-domain --lib` | Passed | `ids` / `relation` 单测已补齐 |
| `cargo test -p memory-config --lib` | Passed | `load` / `from_file` / 默认值 / 错误路径已补齐 |
| `cargo test -p memory-observability --lib` | Passed | metrics / init / span / latency 分支已补齐 |
| `cargo test -p memory-store-pg --lib` | Passed | lazy pool 错误路径与空查询短路已覆盖 |
| `cargo test -p memory-models --lib` | Passed | registry / vision / image helper / 错误分支已补齐 |
| `cargo test -p memory-worker --bin memory-worker` | Passed | event select / event handle / registry error 分支已覆盖 |
| `cargo test -p memory-extract --lib` | Passed | entity / relation / 中文上下文样例与边界提取路径已补齐 |
| `cargo test -p memory-store-md --lib` | Passed | episode repo 错误路径与换行分支已覆盖 |

## Coverage Artifacts

| Artifact | Path |
|---|---|
| Latest Unit Coverage Summary | `target/coverage/unit-pass5/summary.json` |
| Latest Unit Coverage Report | `target/coverage/unit-pass5/report.txt` |
| Latest Unit Raw Summary | `target/coverage/unit-pass5/summary-raw.json` |
| Latest Unit Raw Report | `target/coverage/unit-pass5/report-raw.txt` |
| Latest Unit Executables | `target/coverage/unit-pass5/executables.txt` |
| Latest Unit Profile Data | `target/coverage/unit-pass5/merged.profdata` |
| Previous Unit Coverage Summary | `target/coverage/unit-pass4/summary.json` |

## Notes

| Item | Detail |
|---|---|
| Counting Rule | Coverage numbers above count only production files under `crates/*/src/**`. |
| Execution Strategy | 为避免当前环境下 `LLVM_PROFILE_FILE` 的相对路径 quirk，本轮继续采用“直接执行插桩测试二进制”的方式收集 profile。 |
| Toolchain Constraint | Workspace 仍在 Rust `1.85.0`，因此覆盖率使用 toolchain 自带 LLVM 工具，而不是 `cargo-llvm-cov`。 |
| P0 Repair | `crates/memory-kernel/Cargo.toml` 已补充 `serde_json.workspace = true`，`memory-kernel` 与工作区测试回归已恢复。 |
| Integration Gap | 现有 PG-backed CLI / HTTP integration coverage 仍受 sandbox 约束。 |
| Sandbox Gap | 本地 sandbox 对测试期 socket bind 仍可能返回 `Operation not permitted`，CLI `serve` happy-path 仍更适合通过 seam 或更宽松环境补测。 |
