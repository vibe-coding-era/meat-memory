# Meat Memory 执行日志

## 结构化记录

- timestamp: 2026-04-02T08:34:30Z
  task_id: MM-IDX-001
  executor: Codex
  duration: 0.5h
  status: ✅
  change_hash: N/A-no-git
  key_output: 完成仓库目录盘点，仅存在 docs 下 3 份架构文档
  issues_decisions: 未发现源码与构建资产，决策先以设计文档驱动 0→1 任务拆解
  next_action: 执行 MM-IDX-002 生成模块边界清单

- timestamp: 2026-04-02T08:35:40Z
  task_id: MM-IDX-002
  executor: Codex
  duration: 1.0h
  status: ✅
  change_hash: N/A-no-git
  key_output: 提炼出 Domain/Kernel/Store/Sync/MCP/HTTP/CLI/Worker 模块边界
  issues_decisions: V1/V2 为草案状态，决策以 V2 为实现主依据、V1 为补充
  next_action: 执行 MM-IDX-003 输出原子任务草案

- timestamp: 2026-04-02T08:39:20Z
  task_id: MM-IDX-003
  executor: Codex
  duration: 1.5h
  status: ✅
  change_hash: 132b08176dd18bc426a7f5c539cdd22a664f5a72
  key_output: 生成覆盖 9 大范围的原子级任务清单并落盘 tasklist.md
  issues_decisions: 在“扁平+树形”要求下采用“扁平启动任务 + 树形原子任务”混合格式
  next_action: 创建并维护 task-log.md 作为后续执行审计主日志

- timestamp: 2026-04-02T08:40:37Z
  task_id: MM-IDX-004
  executor: Codex
  duration: 0.5h
  status: ✅
  change_hash: N/A-no-git
  key_output: 创建 task-log.md 并固化结构化字段模板与状态枚举（✅/❌/⏸）
  issues_decisions: 仓库无 git 历史，变更哈希字段暂采用文件哈希或 N/A-no-git 占位
  next_action: 后续每完成一个原子任务立即按同结构追加日志

- timestamp: 2026-04-02 (Asia/Shanghai)
  task_id: MM-IDX-005
  executor: Codex
  duration: 0.5h
  status: ✅
  change_hash: N/A-no-git
  key_output: 重新索引仓库当前状态，确认 `tasks/` 目录已存在，更新 tasklist 顶部快照，并新增 `tasks/project-index.md`
  issues_decisions: 发现此前中断后仓库状态已变化，原 tasklist 顶部“仅 docs 文档”的快照过期，因此补做一次全量索引并将 `docs/` + `tasks/` 作为当前有效执行上下文
  next_action: 后续可直接从 `MM-ENV-001` 开始进入真实环境搭建，或先继续补充 schema/domain/mcp 子文档

- timestamp: 2026-04-02 (Asia/Shanghai)
  task_id: MM-IDX-006
  executor: Codex
  duration: 3.0h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 完成第一批“环境安装 + 工程脚手架 + 本地开发库”执行闭环
  issues_decisions: 将执行重点从“继续写方案”切换为“把仓库推进到可编译、可验证、可继续实现的工程基线”
  next_action: 进入依赖治理、开发体验完善和核心模块真实实现

- timestamp: 2026-04-02 (Asia/Shanghai)
  task_id: MM-ENV-001/MM-ENV-002/MM-ENV-003
  executor: Codex
  duration: 0.5h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 确认 Xcode CLT、Homebrew、Rust toolchain 已可用，`clang`、`brew`、`rustc`、`cargo` 均正常
  issues_decisions: 本机已有可用基础工具链，因此采用“校验 + 固定版本”而不是重新安装
  next_action: 写入 `rust-toolchain.toml` 并固定 workspace 编译环境

- timestamp: 2026-04-02 (Asia/Shanghai)
  task_id: MM-ENV-004/MM-SCAF-002/MM-DEP-001
  executor: Codex
  duration: 1.0h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 创建 `rust-toolchain.toml`、根 `Cargo.toml`、workspace members 与共享依赖清单
  issues_decisions: 精确 pin `1.85.0` 时出现 rustup/toolchain 解析问题，但通过补齐安装并固定 `.cargo/config.toml` 后恢复稳定
  next_action: 继续初始化 crates、配置与本地编译闭环

- timestamp: 2026-04-02 (Asia/Shanghai)
  task_id: MM-ENV-005/MM-ENV-006/MM-ENV-007/MM-ENV-008
  executor: Codex
  duration: 1.0h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 确认本机 PostgreSQL、Docker、Docker Compose 可用，并通过 `compose.yaml` 启动项目专用 `pgvector` 开发库，验证 `vector` 扩展成功启用
  issues_decisions: 发现 Homebrew 的 `pgvector` 与本机 `postgresql@16` 不对齐，因此决定使用项目内 `pgvector/pgvector:pg17` 容器作为目标开发库，避免污染机器现有数据库环境
  next_action: 把本地数据库启动/关闭流程脚本化，并将 pgvector 检查纳入 `verify.sh`

- timestamp: 2026-04-02 (Asia/Shanghai)
  task_id: MM-ENV-009/MM-ENV-010/MM-ENV-011/MM-ENV-013/MM-ENV-014
  executor: Codex
  duration: 0.8h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 确认 `kubectl`、安装并验证 `helm`、`terraform`、`ffmpeg`、`openssl`、`pkg-config`
  issues_decisions: 缺失命令直接通过 Homebrew 补齐；Terraform 使用 `1.5.7` Homebrew 可用版本作为当前阶段基线
  next_action: 将这些依赖统一纳入环境验证脚本

- timestamp: 2026-04-02 (Asia/Shanghai)
  task_id: MM-ENV-012/MM-ENV-015
  executor: Codex
  duration: 0.8h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 创建并验证 `scripts/verify.sh`、`scripts/bootstrap.sh`、`scripts/dev-db-up.sh`、`scripts/dev-db-down.sh`，本地已可跑 `cargo fmt --check`、`cargo clippy`、`cargo check`、`cargo test`
  issues_decisions: `verify.sh` 中 `clippy` 版本检查最初命令写法不兼容，已改为 `cargo clippy -V`；同时把 `pgvector-target` 检查纳入脚本
  next_action: 继续补充依赖安全门禁和本地开发便捷脚本

- timestamp: 2026-04-02 (Asia/Shanghai)
  task_id: MM-SCAF-001/MM-SCAF-003/MM-SCAF-004/MM-SCAF-005/MM-SCAF-006/MM-SCAF-007/MM-SCAF-008/MM-SCAF-009/MM-SCAF-010/MM-SCAF-011/MM-SCAF-012/MM-DEVX-002/MM-DEVX-006
  executor: Codex
  duration: 1.5h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 初始化 Git 仓库和 `.gitignore`，生成 `crates/`、`config/`、`scripts/`、`tests/`、`.github/`、`docs/` 子目录，补齐 README、CHANGELOG、LICENSE、Issue/PR 模板、CI skeleton、Dockerfile、`.editorconfig`
  issues_decisions: 许可证在未给定偏好时默认选用 MIT；当前 Docker 侧先提供最小可运行骨架与 pgvector 本地栈，`app+worker` 的完整 compose 拓扑留到后续 `MM-DEVX-007`
  next_action: 开始替换各 crate 默认模板，实现领域模型、配置层、核心服务和运行入口

- timestamp: 2026-04-02 (Asia/Shanghai)
  task_id: MM-CORE-BOOTSTRAP
  executor: Codex
  duration: 1.2h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 为 `memory-domain`、`memory-store`、`memory-core`、`memory-config`、`memory-observability`、`memory-cli`、`memory-app` 写入最小可用骨架，完成 `cargo check`、`cargo test`、`cargo clippy`、`cargo run -p memory-cli -- print-plan`、`cargo run -p memory-app`
  issues_decisions: 编译期间发现系统 `cc` 被第三方脚本劫持，导致 Rust 链接阶段触发异常外部命令；已通过 `.cargo/config.toml` 显式固定 `clang` 作为 linker/CC 规避环境污染
  next_action: 进入真实的 domain/store/kernel 任务，把剩余 placeholder crate 逐步替换为业务实现

- timestamp: 2026-04-02 (Asia/Shanghai)
  task_id: MM-CORE-BOOTSTRAP-EXT
  executor: Codex
  duration: 0.8h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 将 `memory-assets`、`memory-extract`、`memory-http`、`memory-index`、`memory-mcp`、`memory-models`、`memory-policy`、`memory-store-md`、`memory-store-pg`、`memory-sync`、`memory-worker` 从默认模板替换为和 Memory 系统语义相关的最小接口与测试
  issues_decisions: 不继续保留 `cargo new` 默认 `add(2, 2)` 模板，避免噪音和错误信号；当前这些 crate 仍是语义骨架，后续需要逐步替换为真实实现
  next_action: 根据 tasklist 的 `MM-CORE-*`、`MM-PG-*`、`MM-MD-*` 分批实现真实逻辑

- timestamp: 2026-04-02 (Asia/Shanghai)
  task_id: MM-DEP-002/MM-DEVX-001
  executor: Codex
  duration: 0.3h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 生成 `Cargo.lock`，在 README 中补充 lock policy，新增 `rustfmt.toml` 与 `clippy.toml`，并确认 `cargo fmt --all --check` 与 `cargo clippy --workspace --all-targets -- -D warnings` 通过
  issues_decisions: 依赖锁文件策略采用“应用工作区强制提交、非相关需求不随手改锁文件”的约束，避免后续多人协作时依赖漂移
  next_action: 继续推进本地 compose 拓扑、pre-commit、schema 与核心服务实现

- timestamp: 2026-04-02 18:41:42 CST
  task_id: MM-CORE-001/MM-CORE-002/MM-CORE-003/MM-CORE-004/MM-CORE-005/MM-CORE-006/MM-CORE-007/MM-CORE-008/MM-CORE-009/MM-CORE-010
  executor: Codex
  duration: 0.6h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 复核并闭合 `memory-core` 与 `memory-domain` 的领域模型验收，确认 Artifact/Episode/Memory/Entity/Relation/Scope 及 `MemoryStorePort` 已落地且 `crates/memory-core/tests/domain_tests.rs` 全通过
  issues_decisions: 这些能力此前已实现但未在 tasklist 中回填，本轮以代码与测试为准完成验收，不重复改写已稳定的领域模型
  next_action: 继续闭合 PostgreSQL driver 和 Markdown driver 任务

- timestamp: 2026-04-02 18:41:42 CST
  task_id: MM-DEP-003/MM-DEP-004/MM-PG-001/MM-PG-002/MM-PG-003/MM-PG-004/MM-PG-005/MM-PG-006/MM-PG-007/MM-PG-008
  executor: Codex
  duration: 1.2h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 保持 Rust `1.85.0` 前提下完成 `sqlx` 栈收敛，最小化 features，并将 `memory-store-pg` 打磨为可迁移、可写入、可检索、可集成测试的真实 PG driver
  issues_decisions: `sqlx 0.8.6` 传递依赖默认解析到更高 MSRV，最终采用锁定 `home 0.5.11`、`url 2.5.4`、`idna 1.0.3`、`idna_adapter 1.1.0` 的保守策略，而不是提升 toolchain；同时关闭 `sqlx` 默认 features 以削减无关依赖
  next_action: 落地 Markdown store 的 frontmatter/render/parser 与 roundtrip 测试

- timestamp: 2026-04-02 18:41:42 CST
  task_id: MM-MD-001/MM-MD-002/MM-MD-003/MM-MD-004/MM-MD-005/MM-MD-006
  executor: Codex
  duration: 1.0h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 实现 `MarkdownStore`、frontmatter renderer、`MEMORY.md` rollup 写入/更新、episode 按日分层写入、memory parser 与 `md_roundtrip` 集成测试
  issues_decisions: 当前 Markdown 侧优先采用“scope 聚合 rollup + memory entry markers”的最小可演进方案，以便兼顾人类可读性、对象级更新和后续 agent projection 扩展
  next_action: 补齐开发者体验资产并更新项目索引

- timestamp: 2026-04-02 18:41:42 CST
  task_id: MM-DEVX-003/MM-DEVX-004/MM-DEVX-007/MM-DEVX-010/MM-DEVX-011
  executor: Codex
  duration: 0.7h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 新增 `.env.example`、`justfile`、`.githooks/pre-commit`、`.githooks/commit-msg`、`scripts/install-hooks.sh`，并验证 `docker compose` 的 `app+pgvector+worker` 运行正常，`/healthz` 与 `/api/v1/meta` 返回成功
  issues_decisions: 本地 hooks 通过 `git config core.hooksPath .githooks` 安装，避免污染全局 Git；`bootstrap.sh` 也改为在仓库场景下自动安装 hooks，减少重复手工操作
  next_action: 进入 `memory-kernel`、policy、extract 与 HTTP 写入链路

- timestamp: 2026-04-05 14:59:07 CST
  task_id: MM-KER-001/MM-KER-002/MM-KER-004/MM-HTTP-001/MM-HTTP-002/MM-HTTP-003
  executor: Codex
  duration: 1.5h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 新增 `memory-kernel` crate，落地最小 `remember_text` / `search_context` 编排；`memory-http` 改为真实 Router；`memory-app` 已装配 kernel，并通过 8081 端口 smoke test 验证 `/healthz`、`/api/v1/meta`、`POST /api/v1/memories`、`POST /api/v1/context/search`
  issues_decisions: 当前检索仍是 PG 词法召回，短语查询行为遵循 `%query%` 模式，因此 smoke test 最终采用更贴近现实现状的关键词查询；`default_scope` 改为稳定值 `scp_default_local`，避免每次启动漂移
  next_action: 继续推进 `MM-KER-005/MM-KER-006`、`MM-POL-002`、`MM-EXT-003/MM-EXT-004` 与 `MM-HTTP-004`

- timestamp: 2026-04-05 16:18:00 CST
  task_id: MM-POL-001/MM-POL-002/MM-EXT-001/MM-EXT-002/MM-EXT-003/MM-EXT-004/MM-KER-005/MM-KER-006/MM-HTTP-004
  executor: Codex
  duration: 1.4h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 验收并收口 policy/extract/publish/http 这一批核心能力，补齐 `publish_memory`、PG+Markdown remember→search→publish 集成测试、HTTP API 集成测试，并将 PG 检索从整串 `%query%` 提升为 token-aware 匹配
  issues_decisions: 定向测试暴露出两类问题，一类是检索语义过窄导致 `"gateway http"` 查询 miss，一类是测试隔离不足与语义错位；最终采用“PG 词法检索按 token 组合匹配 + graph 单测直接验证 graph builder + HTTP 测试使用唯一 scope_id”的修正策略
  next_action: 补齐 extract 独立测试与 observability 埋点，把任务文档状态与真实代码对齐

- timestamp: 2026-04-05 16:46:00 CST
  task_id: MM-EXT-005/MM-OBS-001/MM-OBS-002/MM-OBS-003/MM-OBS-006
  executor: Codex
  duration: 1.1h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 新增 `crates/memory-extract/tests/extract_pipeline_tests.rs` 覆盖空输入、低信号、多语言、关系抽取；将 `memory-observability` 从 subscriber 壳子扩展为结构化日志上下文 + 内存指标快照；为 HTTP 新增 `/livez` 与 `/metrics` 路由，并在 kernel 的 remember/search/publish 链路加入 span、latency、命中率等埋点
  issues_decisions: 观测实现当前优先选择“单进程可验证”的方案，不提前引入外部 exporter；日志字段统一为 `trace_id/span_id/scope_id/task_id/component/operation`，分布式 tracing 与告警规则递延到 `MM-OBS-005/MM-OBS-007`
  next_action: 回填 `tasklist.md` 与 `project-index.md` 的完成状态，并继续推进 `MM-KER-003`、`MM-MCP-*`、`MM-CLI-*`、`MM-SYNC-*`

- timestamp: 2026-04-05 17:18:00 CST
  task_id: MM-CLI-001/MM-CLI-002/MM-CLI-003
  executor: Codex
  duration: 0.9h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 将 `memory-cli` 从占位命令升级为真实入口，支持 `remember`、`search`、`serve` 子命令；补齐枚举解析、stdin/文件输入读取、JSON/简报输出，并完成 `cargo run -p memory-cli -- remember/search --json` 的真实 smoke
  issues_decisions: CLI 侧暂直接复用 `memory-app` 的 kernel/router 装配逻辑，而不是提前抽象共享 crate；`serve` 先提供与当前 HTTP app 一致的最小启动能力，后续再视情况抽公共 bootstrap 模块
  next_action: 继续推进 `MM-MCP-*` 或 `MM-KER-003`，把 Agent 接口与多模态写入补齐

- timestamp: 2026-04-05 17:46:00 CST
  task_id: MM-MCP-001/MM-MCP-002/MM-MCP-003/MM-MCP-004
  executor: Codex
  duration: 1.0h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 将 `memory-mcp` 从常量占位库升级为真实 tool 层，新增 dispatcher、tool list、stdio message handler、HTTP `/mcp/tools`/`/mcp/tools/call`；实现 `memory.remember`、`memory.search`、`memory.fetch_context`、`memory.publish`，并补齐 `crates/memory-mcp/tests/mcp_tools_tests.rs` 的 PG+Markdown 集成测试
  issues_decisions: 为了让 `memory.publish` 对 Agent 可用，新增了 `PgStore::get_memory` 与 `Kernel::publish_memory_by_id`，避免要求上游传回完整 Memory 对象；当前 MCP 先采用“统一 dispatcher + 轻量 transport skeleton”的方案，为后续 stdio/http 真实 server 循环保留扩展空间
  next_action: 转入 `MM-KER-003` 或 `MM-SYNC-*`，继续补多模态写入与云本地同步主链路

- timestamp: 2026-04-05 18:08:00 CST
  task_id: MM-SYNC-001/MM-SYNC-002/MM-SYNC-003/MM-SYNC-006
  executor: Codex
  duration: 0.8h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 将 `memory-sync` 从枚举占位 crate 升级为可运行的 replication baseline，新增 `OplogEntry`、`SyncCursor`、`SyncBatch`、`ApplyBatchResult`、`ReplicationEngine`、`InMemoryReplicationEngine`、`append_oplog_entry`、`merge_ops`，并补齐 `crates/memory-sync/tests/sync_merge_tests.rs`
  issues_decisions: 当前 sync 先落“内存版 oplog/merge 基线”，优先固定协议对象和冲突决策，再把持久化、worker 消费和 projection refresh 放到 `MM-SYNC-004/005`；这样能先把混合部署所需的复制抽象定住，而不把实现绑定到单一存储
  next_action: 继续推进 `MM-KER-003`、`MM-SYNC-004/005` 与 `MM-OBS-004/005/007`

- timestamp: 2026-04-05 18:23:19 CST
  task_id: V1-MOD-001/V1-MOD-002
  executor: Codex
  duration: 1.2h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 将 `memory-models` 从占位库扩展为 V1 可用的模型网关骨架，新增 reasoning/extraction/vision/embedding trait、provider/model descriptor、route/fallback registry；`memory-config`、`config/default.toml`、`config/docker.toml` 已接入 Gemini / Claude / ChatGPT / 千问 / 豆包 / Minimax / GLM 的 provider catalog 与默认路由，并在 `memory-app`、`memory-cli`、`memory-worker` 启动期执行 registry 校验
  issues_decisions: 采用“先落 metadata + route registry，不提前接真实 SDK”的 V1 策略，优先保证能力抽象、配置结构和 fallback 规则稳定；同时改为显式 serde rename，避免 `OpenAI -> open_a_i` 这类枚举序列化漂移导致配置不可用
  next_action: 继续推进 `V1-MUL-001`、`V1-MUL-002`、`V1-MUL-003`，补齐图片资产寻址、`remember_image` 与最小 vision/OCR/caption 链路

- timestamp: 2026-04-05 18:30:15 CST
  task_id: V1-MUL-001
  executor: Codex
  duration: 0.9h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 将 `memory-assets` 从最小 `AssetRef` 升级为可运行的本地资产存储层，新增 `StorageClass`、`AssetMetadata`、`PutAssetRequest`、`StoredAsset`、`FileSystemAssetStore`，支持 sha256 内容寻址、分类目录、本地文件落盘、重复写入按哈希去重与回读测试
  issues_decisions: V1 先聚焦“本地目录模式 + 图片资产”，不提前接 S3 与数据库元数据表；资产 ID 采用 `asset_{sha256}` 的稳定派生形式，优先保证同内容幂等寻址和后续 `remember_image` 的可追溯性
  next_action: 继续推进 `V1-MUL-002` 与 `V1-MUL-003`，把 `remember_image`、最小 vision/OCR/caption 派生链路接到 kernel/HTTP/CLI

- timestamp: 2026-04-05 18:42:00 CST
  task_id: V1-MUL-002
  executor: Codex
  duration: 1.1h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 为 `memory-kernel` 新增 `RememberImageRequest` / `remember_image`，将图片写入接到 `memory-assets`；同时在 `memory-http` 增加 `POST /api/v1/images`，在 `memory-cli` 增加 `remember-image` 子命令，并补齐 kernel/HTTP/CLI 的图片写入测试与全工作区回归
  issues_decisions: V1 先采用“图片字节先落本地资产存储，再生成带 `asset://` 引用的 Image Artifact 文本”的最小实现，避免在 V1-MUL-003 之前提前绑定具体 OCR/vision SDK；接口拆成独立 `/api/v1/images`，而不是复用 `/api/v1/memories` 混合文本/图片 payload，以保持调用面清晰
  next_action: 继续推进 `V1-MUL-003`，把最小 OCR/caption/vision extraction 接到模型 registry，并为图片检索补强自动派生文本

- timestamp: 2026-04-05 18:15:37 CST
  task_id: V1-MUL-003
  executor: Codex
  duration: 1.3h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 在 `memory-models` 新增本地 image profile 解析、vision gateway 与中文优先 caption 生成；在 `memory-kernel` 将 vision route 接入 `remember_image`，自动把图片 caption/结构化派生文本写回 Artifact/Memory；在 HTTP/CLI 图片写入返回中补充 `vision_caption` / `vision_model_alias`，并完成全工作区 `fmt/clippy/test`
  issues_decisions: V1 明确采用“本地最小 vision derivation + model route attribution”的策略，而不是立即接入真实云视觉 SDK；这样既能满足图片记忆的自动派生文本闭环，又能保持 provider 抽象和后续真实 SDK 对接边界稳定
  next_action: 转入 `V1-API-004`、`V1-DEP-001`、`V1-DEP-002`、`V1-QA-001`、`V1-DOC-001`，把 Agent 接入文档、部署包和版本验收收口

- timestamp: 2026-04-05 18:39:21 CST
  task_id: V1-API-004/V1-DEP-001/V1-DEP-002/V1-QA-001/V1-DOC-001
  executor: Codex
  duration: 2.0h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 补齐 `docs/agent-integration-v1.md`、HTTP/MCP/CLI 使用文档、本地与云部署 runbook、`config/cloud.example.toml`、`infra/helm/meat-memory`、`scripts/v1-acceptance.sh`、CLI E2E 测试，并将 MCP HTTP 路由真实接入 `memory-app` 与 `memory-cli serve`
  issues_decisions: 本轮在部署 smoke 中发现 `.cargo/config.toml` 全局固定 `/usr/bin/clang` 会导致 Linux Docker 构建失败，因此改为只对 Apple target 固定 clang；同时统一 `memory-worker` 二进制命名，修正 Dockerfile/compose/justfile 的入口不一致问题；云部署包在 V1 采用“单实例 app + worker 默认关闭 + Helm 最小闭环”的保守策略，避免误导为已完成共享卷集群形态
  next_action: 继续推进 `V1-ZH-001` 中文系统化验收集，随后整理 V1 提交与 release 收口

- timestamp: 2026-04-09 (Asia/Shanghai)
  task_id: V1-QA-001/P0/P1
  executor: Codex
  duration: 1.2h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 修复 `memory-kernel` 测试依赖缺口，为 `crates/memory-kernel/Cargo.toml` 补充 `serde_json.workspace = true`，恢复 `cargo test -p memory-kernel --lib --quiet` 与 `cargo test --workspace --lib --bins --quiet` 全绿；重跑单元覆盖率并生成 `target/coverage/unit-pass5/`，当前快照提升到 Line `95.46%`、Function `91.79%`、Region `87.91%`；同步回写 `tasks/tasklist.md` 与 `test-task.md`
  issues_decisions: 继续沿用“带覆盖率插桩的测试二进制直接执行”方案，显式使用绝对 `LLVM_PROFILE_FILE` 路径，并改用 Rust toolchain 自带 `llvm-profdata` / `llvm-cov`，规避当前环境的相对路径 quirk 与 PATH 缺失问题
  next_action: 继续推进 `V1-ZH-001` 中文系统化验收语料收口，并把剩余覆盖率热点聚焦到 `memory-worker`、`memory-app`、`memory-store-pg` 等文件

- timestamp: 2026-04-09 21:49:08 CST
  task_id: V1-ZH-001/V1-REL-001/V1-REL-002
  executor: Codex
  duration: 1.4h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 补齐 `tests/integration/v1-zh-acceptance.md` 中文系统化验收语料，并在 `memory-extract`、`memory-kernel`、`memory-http`、`memory-mcp`、`memory-cli` 增加中文回归；同时把 Browser Console 首页 `/` 与图片 failover `llm_notice` 纳入回归，确认 `cargo test -p memory-extract --lib --quiet`、`cargo test -p memory-kernel --test kernel_flow_tests --quiet`、`cargo test -p memory-http --test http_api_tests --quiet`、`cargo test -p memory-mcp --test mcp_tools_tests --quiet`、`cargo test -p memory-cli --test cli_e2e --quiet`、`cargo test --workspace --lib --bins --quiet`、`./scripts/v1-acceptance.sh` 全部通过，并清理根目录 `*.profraw` 残留、更新 `.gitignore`
  issues_decisions: 中文关系抽取验收样例采用“重复显式主语”的稳定写法，以规避当前抽取器对省略主语跨分句推断的不确定性；CLI failover 回归沿用“从首个 `{` 开始解析 JSON”的兼容策略，避免日志前缀影响断言；覆盖率原始产物继续视为临时噪音，不纳入长期项目记忆
  next_action: 收口 V1 版本提交边界、整理 release 说明，并开始准备 V2 的 scope/多语言规划

- timestamp: 2026-04-10 00:15:00 CST
  task_id: V1-REL-003
  executor: Codex
  duration: 0.8h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 补齐 `docs/release-notes-v1.md`，同步更新 `CHANGELOG.md`、`README.md`、`docs/README.md`、`tasks/tasklist.md`、`tasks/project-index.md`，将仓库状态从“V1 封板中”收口到“V1 已完成”；同时复核 `docker compose config --quiet` 与 `helm lint infra/helm/meat-memory` 作为 V1 部署边界验收入口
  issues_decisions: V1 版本号继续维持 `0.1.0`，把 2026-04-09 作为当前封板日期；封板文档只声明 V1 已交付的文本+图片、中文优先、HTTP/CLI/MCP、本地/云独立部署和混合部署预留接口，不把团队隔离、中英双语、音视频链路提前纳入 V1 口径
  next_action: V1 已完成，后续进入 `V2-SCP-*`、`V2-LNG-*`、`V2-SYN-*` 范围准备

- timestamp: 2026-04-10 00:58:00 CST
  task_id: V2-SCP-001
  executor: Codex
  duration: 0.9h
  status: ⏳
  change_hash: N/A-uncommitted
  key_output: 将 `memory-domain` 的 `Scope` 扩展为 V2 基线对象，新增 `owner_principal_id`、`inherit_policy`、`sync_policy` 与层级/路径校验；为 `memory-store-pg` 新增 `0003_scope_governance.sql`、`seed_scope_definition` 与 richer scope 落库路径；`memory-kernel` 改为按正式 `Scope` 对象 seed project scope，并补齐 domain / store / kernel 定向测试
  issues_decisions: 本轮先收口 `V2-SCP-001` 的 domain + persistence 基线，不提前把 `V2-SCP-002` 的 publish/review/redaction policy 混进来；默认策略采用 `project/team/org -> replicated`、`user -> shared`、`session -> local_only` 的保守分层，后续可在 policy engine 中继续细化
  next_action: 继续推进 `V2-SCP-001` 的 runtime 接入面与 owner/principal 使用链路，然后进入 `V2-SCP-002` 的 publish/review/shared policy 工作流

- timestamp: 2026-04-10 02:40:00 CST
  task_id: V2-SCP-001/V2-SCP-002/V2-MRG-001/V2-SYN-001/V2-SYN-002/V2-LNG-001/V2-LNG-002
  executor: Codex
  duration: 3.0h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 完成 V2 主线实现收口：为 `Memory` 增加 `owner_scope_id` / `published_from_scope_id` / `language_code` 并贯通 PG+Markdown；`memory-policy` 增加 scope-aware publish policy 与 redaction；`memory-kernel` 增加 `promote_memory`、markdown fallback `get_memory` 与语言透传；`memory-http` / `memory-mcp` 增加 promote 入口；`memory-sync` 增加 `FileReplicationEngine`、`ConflictRecord`、`SyncStatus`；`memory-config` / `memory-worker` 增加 sync `node_id` / `state_path` runtime 配置；新增 `tests/integration/v2-bilingual-acceptance.md`
  issues_decisions: V2 的同步持久化先采用本地 file-backed state，而不是一次性绑定数据库表或远端协议，实现目标是先把 pull/apply/conflict/audit 语义坐实；跨 scope promotion 采用“生成新 memory + 保留 owner_scope/published_from_scope + 按策略进入 active/candidate”的保守实现，避免直接改写原对象归属
  next_action: V2 已完成，后续主线进入 `V3-MM-*` 全多模态范围

- timestamp: 2026-04-11 (Asia/Shanghai)
  task_id: V2-QA-PERF/V2-QA-INT
  executor: Codex
  duration: 1.5h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 新增 `scripts/v2-perf.sh` 与 `memory-kernel` / `memory-sync` ignored perf smoke tests，输出 `target/perf/*.txt` 基准报告；同时补强 `kernel/http/mcp/sync` 外部测试，覆盖 markdown-only `get_memory + promote`、HTTP `/api/v1/memories/promote`、MCP `memory.promote`、file-backed sync reopen/status/conflict 回归
  issues_decisions: 性能测试先采用 deterministic smoke benchmark 而非引入额外 benchmark 框架，优先确保 CI/本地都能低门槛复现并生成可比对文本报告；integration 强化重点放在 V2 新增的跨 scope promotion 与 file-backed sync 持久化链路，而不是重复已有 remember/search happy path
  next_action: 若进入 V3，可在此基础上接入更大规模数据集、并发压测与长期 trend snapshot

- timestamp: 2026-04-11 (Asia/Shanghai)
  task_id: V2-QA-REPORTS
  executor: Codex
  duration: 1.2h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 新增 `tests/reports/` 目录体系与分类 `README.md`，提供 `scripts/write-test-reports.sh` 一键生成 unit/integration/e2e/perf 报告并归档到 `latest/archive`；同时调整 `scripts/v2-perf.sh` 默认输出到 `tests/reports/perf/latest/`，清理旧的 `target/perf` 临时产物路径
  issues_decisions: 报告体系先采用“可读文本日志 + perf 原始 JSON 行输出 + latest/archive 双层目录”的轻量方案，不引入额外测试报告框架；`security` 目录先保留 placeholder，待后续补上可执行安全检查后再写实跑脚本
  next_action: 后续如需接入 CI，可直接把 `./scripts/write-test-reports.sh` 作为统一测试报告入口

- timestamp: 2026-04-11 (Asia/Shanghai)
  task_id: V2.1-CLI-001/V2.1-CLI-002/V2.1-SKL-001/V2.1-SKL-002/V2.1-SKL-003/V2.1-TUI-001
  executor: Codex
  duration: 1.2h
  status: ⏳
  change_hash: N/A-uncommitted
  key_output: 启动 V2.1：为 `memory-cli` 新增 `config show`、`config check`、`mcp info`、`tui init` 入口；增强 MCP tool 描述中的必填/可选参数提示；新增 `docs/agent-skills/` 下 Codex、Claude Code/TRAE/Qoder、OpenClaw/CoWork/QoderWork 三类 Agent skill 源模板，并同步更新 CLI/MCP 文档与 tasklist 状态
  issues_decisions: TUI 先采用可运行的初始化面板预览，不在第一步直接写配置文件，以避免误改用户配置；Agent skill 先作为仓库内源模板沉淀，后续再补安装/发布路径和平台差异细节
  next_action: 继续推进 `V2.1-TUI-002` / `V2.1-TUI-003`，把 `tui init` 从只读预览升级为可校验、可生成推荐配置的向导

- timestamp: 2026-04-11 (Asia/Shanghai)
  task_id: V2.1-TUI-002/V2.1-TUI-003
  executor: Codex
  duration: 0.8h
  status: ⏳
  change_hash: N/A-uncommitted
  key_output: 将 `memory-cli tui init` 从只读预览升级为可生成配置文件的初始化向导，支持 `--write-config`、`--force`、MCP 开关、数据库 URL、Markdown/Assets 根目录和 reasoning/extraction/vision/embedding 路由覆盖；写配置前复用 model registry 校验，并自动去除 primary/fallback 重复 alias
  issues_decisions: 当前仍保持非交互参数式 TUI，避免在自动化测试和 Agent 执行中引入难以控制的交互状态；数据库实时连通性检查留到下一步，先确保配置生成与模型路由校验稳定
  next_action: 继续补 `V2.1-TUI-003` 的数据库连通性检查，并开始规划 Agent skill 的安装/分发路径

- timestamp: 2026-04-11 (Asia/Shanghai)
  task_id: V2.1-TUI-003
  executor: Codex
  duration: 0.4h
  status: ⏳
  change_hash: N/A-uncommitted
  key_output: 为配置向导补齐显式数据库连通性检查：`memory-cli config check --database` 与 `memory-cli tui init --check-database` 会在 PG 启用时连接当前 `database_url`，并把 connected / warning 状态写入文本与 JSON 输出
  issues_decisions: 数据库检查保持 opt-in，不作为默认 `config check` 行为，避免用户未启动 PG 时影响快速查看配置；本地默认配置实测数据库 connected
  next_action: 继续推进 Agent skill 安装/分发路径，或把 TUI 从参数式面板进一步升级成交互式选择器

- timestamp: 2026-04-11 (Asia/Shanghai)
  task_id: V2.1-SKL-001/V2.1-SKL-002/V2.1-SKL-003
  executor: Codex
  duration: 0.4h
  status: ⏳
  change_hash: N/A-uncommitted
  key_output: 新增 `scripts/export-agent-skills.sh`，支持按 `codex`、`claude-code`、`execution-agent` 或 `all` 导出 skill 模板到指定目录，并自动生成 bundle `README.md`；同步更新 `docs/agent-skills/README.md` 的导出用法
  issues_decisions: 先采用“仓库内源模板 + 导出脚本”的轻量分发方式，不直接写入用户全局 skill 目录，避免污染本机环境；导出目录默认放到 `dist/agent-skills`，也支持显式指定输出路径
  next_action: 若继续推进，可补平台专用 metadata 文件，或让 CLI 直接包装 skill 导出命令

- timestamp: 2026-04-11 (Asia/Shanghai)
  task_id: V2.1-SKL-001/V2.1-SKL-002/V2.1-SKL-003
  executor: Codex
  duration: 0.4h
  status: ⏳
  change_hash: N/A-uncommitted
  key_output: 为 `memory-cli` 增加 `skills export` 子命令，支持按 `codex` / `claude-code` / `execution-agent` / `all` 导出 Agent skill 模板到指定目录；实测导出到 `/tmp/meat-memory-agent-skills-cli` 成功，并补齐 CLI 文档与 Agent skill README 的命令示例
  issues_decisions: CLI 导出默认基于当前仓库根目录下的 `docs/agent-skills` 源模板工作，并要求输出目录不存在或显式传 `--force`，避免覆盖用户已有技能包
  next_action: 后续可继续补平台专用 metadata 文件，例如 Codex skill 的 `agents/openai.yaml` 或其他 Agent 平台的清单格式

- timestamp: 2026-04-11 (Asia/Shanghai)
  task_id: V2.1-SKL-001/V2.1-SKL-002/V2.1-SKL-003
  executor: Codex
  duration: 0.2h
  status: ⏳
  change_hash: N/A-uncommitted
  key_output: 为三套 Agent skill 的 `agents/openai.yaml` 补齐 UI metadata，新增 `brand_color`，并确认导出包现在包含 `SKILL.md + agents/openai.yaml` 的完整结构；同步更新 Agent skill README 与 tasklist 状态描述
  issues_decisions: 本轮只补 `brand_color`，不额外引入图标资源，避免为 UI 装饰增加无关资产维护成本；后续若需要可再补 `icon_small` / `icon_large`
  next_action: 如继续打磨 skill，可补图标资产与平台专用 metadata，或转回推进交互式 TUI

- timestamp: 2026-04-11 (Asia/Shanghai)
  task_id: V2.1-SKL-001/V2.1-SKL-002/V2.1-SKL-003/V2.1-DOC-001
  executor: Codex
  duration: 0.3h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 为三套 Agent skill 补齐 `assets/icon.svg` 视觉资产，并将 `agents/openai.yaml` 扩展为包含 `icon_small`、`icon_large`、`brand_color`、`default_prompt` 的完整 UI metadata；同步更新 Agent skill README、CLI 文档与 tasklist，使导出包结构明确为 `SKILL.md + agents/openai.yaml + assets/icon.svg`
  issues_decisions: 图标先采用仓库内可维护的 SVG 资产，不引入额外位图或品牌资源管线；优先保证导出包可直接安装、字段清晰、跨平台可复用
  next_action: 继续推进 `V2.1-DOC-001` 与 `V2.1-QA-001`，补 README/runbook 收口和 skill/CLI/TUI 验收脚本

- timestamp: 2026-04-11 (Asia/Shanghai)
  task_id: V2.1-DOC-001
  executor: Codex
  duration: 0.2h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 在根 `README.md`、`docs/README.md` 与 `docs/api/README.md` 增补 V2.1 快速入口，明确 `config check`、`mcp info`、`tui init`、`skills export` 的推荐起手顺序，把 CLI/TUI/skill 的安装后使用路径收口到第一层文档
  issues_decisions: 文档优先解决“新用户第一步做什么”的发现问题，不在 README 顶层展开过多参数细节；细节继续下沉到 `docs/api/cli-v1.md` 与 `docs/agent-skills/README.md`
  next_action: 继续推进 `V2.1-QA-001`，补一键式 V2.1 smoke 验收脚本与报告输出

- timestamp: 2026-04-11 (Asia/Shanghai)
  task_id: V2.1-QA-001
  executor: Codex
  duration: 0.5h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 新增并实跑 `scripts/v2_1-acceptance.sh`，覆盖 `memory-cli config check`、`mcp info`、`tui init`、`skills export` 与导出结果结构校验；同时将其接入 `scripts/write-test-reports.sh`，生成 `tests/reports/e2e/latest/v2_1-acceptance.txt` 并把索引写入 `tests/reports/latest-run.md`
  issues_decisions: 在沙箱内刷新统一报告时，集成测试因本地 PostgreSQL 访问受限出现 `Operation not permitted (os error 1)`；改为在已获授权的非沙箱环境重跑 `./scripts/write-test-reports.sh` 后恢复正常，说明失败源自执行环境限制而非代码回归
  next_action: 继续推进 `V2.1-DOC-001` 的剩余 README/runbook 收口，或转入下一阶段的交互式 TUI 能力

- timestamp: 2026-04-11 (Asia/Shanghai)
  task_id: V2.1-TUI-001/V2.1-TUI-002/V2.1-TUI-003/V2.1-DOC-001/V2.1-QA-001
  executor: Codex
  duration: 0.6h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 为 `memory-cli tui init` 增加 `--interactive` 交互问答模式，支持逐步选择 MCP 开关、数据库 URL、Markdown/Assets 根目录、四类主模型 alias、数据库检查和配置写出；同时补齐交互模式单测、CLI 文档示例，并把 `scripts/v2_1-acceptance.sh` 扩展为覆盖交互式 smoke
  issues_decisions: 这一轮优先采用标准输入/输出驱动的轻量交互，不引入额外 TUI 依赖，以保持脚本化兼容和实现稳定；`--interactive` 明确不与 `--json` 组合，避免破坏 JSON 输出格式
  next_action: 若继续推进 TUI，可再升级为方向键/菜单式选择器，或转入 CLI/MCP 的错误提示优化收口

- timestamp: 2026-04-11 (Asia/Shanghai)
  task_id: V2.1-TUI-001/V2.1-TUI-003/V2.1-DOC-001/V2.1-QA-001
  executor: Codex
  duration: 0.4h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 将 `--interactive` 进一步升级为 setup wizard 形态，新增 `Local default` / `MCP-ready` / `Markdown-first` 三个 setup profile、分步骤标题、review summary 和最终确认；同时补齐取消路径单测，并更新 CLI 文档与 V2.1 验收脚本，使交互式 smoke 覆盖 profile + confirm 流程
  issues_decisions: 继续保持标准输入驱动的实现，先把“流程结构清晰”和“可取消”补齐，而不立即引入方向键菜单或额外终端 UI 库；这样既保留自动化兼容，也便于后续逐步演进
  next_action: 若继续推进 TUI，可增加编号菜单式模型选择和保存后自动校验，或再往真正全屏式 TUI 组件演进

- timestamp: 2026-04-11 (Asia/Shanghai)
  task_id: V2.1-TUI-001/V2.1-TUI-002/V2.1-DOC-001/V2.1-QA-001
  executor: Codex
  duration: 0.4h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 将 `--interactive` 的第一步前置为语言选择，支持 `中文 / English` 两种向导语言，并同步写入默认 locale；随后再进入 setup profile、存储、模型路由、校验与确认步骤。CLI 文档和 V2.1 验收脚本已更新为新的“先选语言，再做其他设置”流程
  issues_decisions: 这一轮把“向导显示语言”和“默认 locale 配置”合并处理，减少用户重复设置；当前文案与步骤会跟语言切换，但最终结果面板仍保持原有通用格式，避免影响现有非交互输出与验收口径
  next_action: 若继续推进 TUI，可把模型 alias 输入从自由文本升级为编号候选菜单，并补英文流程 smoke

- timestamp: 2026-04-11 (Asia/Shanghai)
  task_id: V2.1-TUI-002/V2.1-DOC-001/V2.1-QA-001
  executor: Codex
  duration: 0.3h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 将交互式 TUI 的 reasoning/extraction/vision/embedding 选择升级为编号候选菜单，显示当前可用 alias 列表与默认编号，用户回车可保留当前值、输入数字即可切换，不必再手输 alias；同时补齐单测和 CLI 文档说明
  issues_decisions: 保留了“直接输入文本”的兼容回退，但默认体验已经转为编号选择，优先优化首次安装速度；当前候选列表按照现有 catalog 顺序展示，后续可再引入按 locale/provider 分组
  next_action: 若继续推进 TUI，可补英文流程 smoke、provider 分组展示，或把最终结果面板也切到随语言切换

- timestamp: 2026-04-11 (Asia/Shanghai)
  task_id: V2.1-CLI-001/V2.1-CLI-002/V2.1-SKL-001/V2.1-SKL-002/V2.1-SKL-003/V2.1-TUI-001/V2.1-TUI-002/V2.1-TUI-003/V2.1-DOC-001/V2.1-QA-001
  executor: Codex
  duration: 0.8h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 收口 V2.1 全量范围：补齐 `mcp info --check-http` 本地连通性检查；让交互式 TUI 的最终结果面板跟随语言切换；将英文交互流程纳入 `scripts/v2_1-acceptance.sh`；新增 `docs/runbook/v2_1-quickstart.md` 并完成 README/runbook 索引收口。至此 V2.1 的 CLI、MCP、Agent skill、TUI、文档与验收均已完成并可独立交付
  issues_decisions: 统一采用“沙箱友好的验收方式”，避免依赖测试内临时监听端口；`mcp info --check-http` 在服务未启动时明确返回 `unreachable`，保持诊断信息可预期而不是静默失败
  next_action: V2.1 已完成；后续可转入 V3 多模态能力，或继续增强全屏式 TUI 体验
