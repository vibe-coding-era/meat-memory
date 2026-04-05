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
