# Meat Memory 0→1 原子任务清单

## 项目索引快照

- 当前仓库目录：`.cargo/`、`.githooks/`、`config/`、`crates/`、`docs/`、`infra/`、`scripts/`、`tasks/`、`tests/`
- 当前核心文档：`docs/meat-memory-prompt.md`、`docs/meat-memory-scheme-v1.md`、`docs/meat-memory-scheme-v2.md`
- 当前执行资产：`tasks/tasklist.md`、`tasks/task-log.md`、`tasks/project-index.md`
- 当前实现状态：工程基线、核心领域模型、PostgreSQL/Markdown 双存储、policy/extract/publish/search kernel、HTTP remember/search/health/metrics 接口、CLI remember/search/serve 入口、MCP remember/search/fetch_context/publish tool 层、内存版 sync oplog/merge baseline、结构化日志与检索指标快照、compose 本地栈、环境模板与 Git hooks 已落地，workspace 已通过 fmt/clippy/test 全量校验
- 目标产物：基于设计稿落地为可运行、可测试、可部署、可观测、可审计的长期 Memory 内核
- 当前建议起点：进入 `MM-KER-003`、`MM-SYNC-004/005` 与 `MM-OBS-004/005/007`，继续把多模态、后台任务和可观测性扩展做实

## 扁平启动任务（已执行）

- [x] MM-IDX-001 Index repository baseline documents (完成 docs 目录全量盘点并识别实现空白) <!-- priority:H est:0.5h dep:none owner:Arch sp:1 risk:遗漏隐藏文件 rollback:二次全量扫描 -->
- [x] MM-IDX-002 Derive implementation module map from V1/V2 schemes (输出 crate 边界与主链路模块清单) <!-- priority:H est:1h dep:MM-IDX-001 owner:Arch sp:2 risk:设计理解偏差 rollback:回退到 V2 术语与流程定义 -->
- [x] MM-IDX-003 Create atomic delivery backlog draft (生成覆盖9大范围的原子任务草案) <!-- priority:H est:1.5h dep:MM-IDX-002 owner:PM sp:3 risk:颗粒度不一致 rollback:按函数级重切分 -->
- [x] MM-IDX-004 Baseline governance metadata templates (定义任务日志字段、状态枚举、执行人规范) <!-- priority:M est:0.5h dep:MM-IDX-003 owner:PM sp:1 risk:字段缺失 rollback:按审计字段最小集重置 -->
- [x] MM-IDX-005 Refresh repository index after interrupted turns (确认 `tasks/` 产物已存在并补齐最新项目索引) <!-- priority:M est:0.5h dep:MM-IDX-004 owner:Arch sp:1 risk:快照过期 rollback:重新全量扫描并发布 project-index -->
- [x] MM-IDX-006 Execute environment-and-scaffold bootstrap tranche (完成第一批环境安装、workspace 初始化、CI skeleton 与本地 pgvector 开发库闭环) <!-- priority:H est:3h dep:MM-IDX-005 owner:Arch sp:5 risk:工具链与系统环境冲突 rollback:固定 linker/toolchain 并退回最小闭环 -->

## 树形任务分解（原子级）

### 1) 运行环境安装与校验

- [x] MM-ENV-001 Install Xcode Command Line Tools on macOS (执行 `xcode-select --install` 后 `clang --version` 可用) <!-- priority:H est:0.5h dep:none owner:DevOps sp:1 risk:系统权限受限 rollback:卸载后重新安装 -->
- [x] MM-ENV-002 Install Homebrew package manager (执行 `brew --version` 返回有效版本) <!-- priority:H est:0.5h dep:MM-ENV-001 owner:DevOps sp:1 risk:镜像源不可达 rollback:切换官方安装脚本 -->
- [x] MM-ENV-003 Install Rust toolchain via rustup (执行 `rustc --version` 与 `cargo --version` 成功) <!-- priority:H est:0.5h dep:MM-ENV-002 owner:DevOps sp:1 risk:版本漂移 rollback:锁定 toolchain channel -->
- [x] MM-ENV-004 Pin Rust toolchain version file (仓库存在 `rust-toolchain.toml` 且团队版本一致) <!-- priority:H est:0.5h dep:MM-ENV-003 owner:BE sp:1 risk:跨机器不一致 rollback:回退到 stable 并重锁版本 -->
- [x] MM-ENV-005 Install PostgreSQL server and client (执行 `psql --version` 且可本地连接) <!-- priority:H est:1h dep:MM-ENV-002 owner:DevOps sp:2 risk:端口冲突 rollback:改用容器化 PG -->
- [x] MM-ENV-006 Enable pgvector extension in target instance (执行 `CREATE EXTENSION vector;` 成功) <!-- priority:H est:0.5h dep:MM-ENV-005 owner:DBA sp:1 risk:扩展不可用 rollback:切换兼容镜像版本 -->
- [x] MM-ENV-007 Install Docker Engine/Desktop (执行 `docker version` 成功) <!-- priority:H est:0.5h dep:MM-ENV-002 owner:DevOps sp:1 risk:虚拟化不可用 rollback:改用 Colima -->
- [x] MM-ENV-008 Install Docker Compose plugin (执行 `docker compose version` 成功) <!-- priority:H est:0.5h dep:MM-ENV-007 owner:DevOps sp:1 risk:插件缺失 rollback:独立安装 compose binary -->
- [x] MM-ENV-009 Install Kubernetes CLI tooling (执行 `kubectl version --client` 成功) <!-- priority:M est:0.5h dep:MM-ENV-002 owner:DevOps sp:1 risk:版本不兼容 rollback:切换稳定版本 -->
- [x] MM-ENV-010 Install Helm CLI (执行 `helm version` 成功) <!-- priority:M est:0.5h dep:MM-ENV-009 owner:DevOps sp:1 risk:repo 配置错误 rollback:清理 helm 缓存并重配 -->
- [x] MM-ENV-011 Install IaC driver Terraform/OpenTofu (执行 `terraform version` 或 `tofu version` 成功) <!-- priority:M est:0.5h dep:MM-ENV-002 owner:DevOps sp:1 risk:provider 锁失败 rollback:固定 provider 版本 -->
- [x] MM-ENV-012 Install CI runner dependencies (本地可模拟 CI 执行 lint/test/build) <!-- priority:M est:1h dep:MM-ENV-003 owner:DevOps sp:2 risk:脚本与 CI 环境差异 rollback:容器化 CI 执行环境 -->
- [x] MM-ENV-013 Validate FFmpeg runtime for multimodal pipeline (执行 `ffmpeg -version` 成功) <!-- priority:M est:0.5h dep:MM-ENV-002 owner:BE sp:1 risk:编解码库缺失 rollback:切换完整构建发行版 -->
- [x] MM-ENV-014 Validate OpenSSL and pkg-config toolchain (执行 `openssl version`、`pkg-config --version` 成功) <!-- priority:H est:0.5h dep:MM-ENV-002 owner:DevOps sp:1 risk:链接失败 rollback:重装依赖并刷新 PATH -->
- [x] MM-ENV-015 Create environment verification script (执行脚本输出所有依赖 PASS/FAIL) <!-- priority:H est:1h dep:MM-ENV-001 owner:DevOps sp:2 risk:漏检关键依赖 rollback:补充校验项并重跑 -->

### 2) 项目脚手架生成与目录规范初始化

- [x] MM-SCAF-001 Initialize git repository metadata baseline (初始化 `.gitignore` 且忽略构建与密钥文件) <!-- priority:H est:0.5h dep:MM-ENV-003 owner:BE sp:1 risk:误提交敏感文件 rollback:重写 ignore 并清理缓存 -->
- [x] MM-SCAF-002 Initialize Rust workspace root Cargo manifest (存在根 `Cargo.toml` 且可识别 workspace members) <!-- priority:H est:1h dep:MM-ENV-004 owner:BE sp:2 risk:成员路径错误 rollback:回退最小 workspace -->
- [x] MM-SCAF-003 Create canonical directory layout (创建 `crates/`, `scripts/`, `config/`, `tests/`, `.github/workflows/`) <!-- priority:H est:1h dep:MM-SCAF-002 owner:BE sp:2 risk:目录命名冲突 rollback:统一重命名并迁移 -->
- [x] MM-SCAF-004 Add LICENSE file for distribution policy (LICENSE 存在并经法务确认) <!-- priority:M est:0.5h dep:MM-SCAF-003 owner:PM sp:1 risk:许可证误选 rollback:替换为目标许可证 -->
- [x] MM-SCAF-005 Add README bootstrap content (README 包含启动、构建、测试、部署最小指南) <!-- priority:H est:1h dep:MM-SCAF-003 owner:TechWriter sp:2 risk:与实现脱节 rollback:按脚本自动生成片段 -->
- [x] MM-SCAF-006 Add CHANGELOG structure (CHANGELOG 遵循版本化格式并可持续追加) <!-- priority:M est:0.5h dep:MM-SCAF-003 owner:TechWriter sp:1 risk:格式不统一 rollback:切换标准模板 -->
- [x] MM-SCAF-007 Create docs index and architecture directory (docs 下建立 architecture/api/runbook 子目录) <!-- priority:M est:0.5h dep:MM-SCAF-003 owner:TechWriter sp:1 risk:文档分散 rollback:重组目录并更新索引 -->
- [x] MM-SCAF-008 Initialize scripts automation entrypoints (创建 `scripts/bootstrap.sh`、`scripts/verify.sh`) <!-- priority:H est:1h dep:MM-SCAF-003 owner:DevOps sp:2 risk:跨平台兼容性 rollback:拆分脚本并降级能力 -->
- [x] MM-SCAF-009 Initialize config templates (创建 `config/default.toml` 与环境覆盖模板) <!-- priority:H est:1h dep:MM-SCAF-003 owner:BE sp:2 risk:配置键漂移 rollback:引入配置 schema 校验 -->
- [x] MM-SCAF-010 Initialize tests harness directories (创建 unit/integration/e2e/perf/security 目录与占位) <!-- priority:H est:0.5h dep:MM-SCAF-003 owner:QA sp:1 risk:测试资产分散 rollback:按层重组目录 -->
- [x] MM-SCAF-011 Initialize GitHub Actions workflow skeleton (存在 `ci.yml`、`release.yml`、`security.yml`) <!-- priority:H est:1h dep:MM-SCAF-003 owner:DevOps sp:2 risk:工作流权限不足 rollback:最小只读权限重建 -->
- [x] MM-SCAF-012 Initialize issue/pr templates (模板可指导缺陷、功能、发布流程) <!-- priority:M est:0.5h dep:MM-SCAF-011 owner:PM sp:1 risk:字段冗余 rollback:裁剪至最小治理字段 -->

### 3) 依赖声明与锁定（含 SBOM/漏洞/license）

- [x] MM-DEP-001 Declare workspace dependencies in root Cargo.toml (依赖集中声明且 member 可继承) <!-- priority:H est:1h dep:MM-SCAF-002 owner:BE sp:2 risk:版本冲突 rollback:回退到 crate 级声明 -->
- [x] MM-DEP-002 Generate Cargo.lock and commit lock policy (生成 `Cargo.lock` 并定义更新策略) <!-- priority:H est:0.5h dep:MM-DEP-001 owner:BE sp:1 risk:锁文件漂移 rollback:固定 resolver 与版本 -->
- [x] MM-DEP-003 Add SQLx and Tokio async stack dependencies (编译通过且 feature 最小化) <!-- priority:H est:1h dep:MM-DEP-001 owner:BE sp:2 risk:feature 冲突 rollback:拆分 feature 集 -->
- [x] MM-DEP-004 Add Axum/HTTP serialization dependencies (HTTP crate 编译通过并具备最小路由能力) <!-- priority:H est:1h dep:MM-DEP-001 owner:BE sp:2 risk:生态版本不兼容 rollback:锁定兼容矩阵 -->
- [ ] MM-DEP-005 Add tracing and observability dependencies (可输出结构化日志与 trace id) <!-- priority:H est:0.5h dep:MM-DEP-001 owner:SRE sp:1 risk:日志格式不统一 rollback:统一 subscriber 初始化 -->
- [ ] MM-DEP-006 Add migration driver dependency (`sqlx-cli` 或等价工具可执行迁移) <!-- priority:H est:0.5h dep:MM-DEP-003 owner:DBA sp:1 risk:迁移版本不一致 rollback:使用容器化迁移执行 -->
- [ ] MM-DEP-007 Add security scanning dependencies (`cargo-audit` 可执行) <!-- priority:H est:0.5h dep:MM-DEP-001 owner:Sec sp:1 risk:漏洞库更新失败 rollback:离线数据库镜像 -->
- [ ] MM-DEP-008 Add license scanning dependency (`cargo-deny` 可执行) <!-- priority:H est:0.5h dep:MM-DEP-001 owner:Sec sp:1 risk:许可证规则误判 rollback:调整 allow/deny 清单 -->
- [ ] MM-DEP-009 Generate SBOM artifact (`cyclonedx` 输出 sbom.json) <!-- priority:M est:1h dep:MM-DEP-001 owner:Sec sp:2 risk:格式不兼容 rollback:切换 SPDX/CycloneDX 版本 -->
- [ ] MM-DEP-010 Integrate vulnerability scan into CI (PR 触发漏洞扫描并阻断高危依赖) <!-- priority:H est:1h dep:MM-DEP-007 owner:DevOps sp:2 risk:误报阻塞开发 rollback:仅阻断可利用高危项 -->
- [ ] MM-DEP-011 Integrate license compliance gate into CI (非允许许可证触发失败) <!-- priority:H est:1h dep:MM-DEP-008 owner:Sec sp:2 risk:历史依赖不兼容 rollback:临时豁免并限期整改 -->
- [ ] MM-DEP-012 Implement dependency update cadence policy (形成周更/双周更流程并可追踪) <!-- priority:M est:0.5h dep:MM-DEP-002 owner:PM sp:1 risk:更新滞后 rollback:启用自动 PR 机器人 -->

### 4) 本地开发工具链配置

- [x] MM-DEVX-001 Add rustfmt and clippy config (执行 `cargo fmt --check` 与 `cargo clippy` 通过) <!-- priority:H est:1h dep:MM-SCAF-002 owner:BE sp:2 risk:规则过严 rollback:分阶段收敛警告 -->
- [x] MM-DEVX-002 Add editorconfig baseline (不同编辑器换行/缩进一致) <!-- priority:M est:0.5h dep:MM-SCAF-003 owner:BE sp:1 risk:历史文件差异 rollback:批量格式化统一 -->
- [x] MM-DEVX-003 Add pre-commit hook for fmt/lint/test smoke (提交前自动执行最小校验) <!-- priority:H est:1h dep:MM-DEVX-001 owner:DevOps sp:2 risk:提交流程变慢 rollback:拆分为可选本地钩子 -->
- [x] MM-DEVX-004 Add commit message convention check (提交信息满足 Conventional Commits) <!-- priority:M est:0.5h dep:MM-DEVX-003 owner:PM sp:1 risk:开发者阻抗 rollback:仅 CI 软校验 -->
- [ ] MM-DEVX-005 Add NVM/ASDF version control files (Node/工具版本可复现) <!-- priority:M est:0.5h dep:MM-ENV-002 owner:DevOps sp:1 risk:多版本冲突 rollback:只保留 asdf 管理 -->
- [x] MM-DEVX-006 Create Dockerfile for local/service runtime (镜像可构建且应用可启动) <!-- priority:H est:1h dep:MM-SCAF-003 owner:DevOps sp:2 risk:体积过大 rollback:多阶段构建优化 -->
- [x] MM-DEVX-007 Create docker-compose local stack (一键启动 app+pg+worker) <!-- priority:H est:1h dep:MM-DEVX-006 owner:DevOps sp:2 risk:服务编排竞态 rollback:添加健康检查和依赖顺序 -->
- [ ] MM-DEVX-008 Configure hot-reload workflow for Rust dev loop (代码变更自动重编译并重启) <!-- priority:M est:1h dep:MM-DEVX-007 owner:BE sp:2 risk:资源占用高 rollback:改回手动重启 -->
- [ ] MM-DEVX-009 Define default port mapping policy (端口冲突检测并可覆盖) <!-- priority:M est:0.5h dep:MM-DEVX-007 owner:DevOps sp:1 risk:与本机服务冲突 rollback:引入动态端口映射 -->
- [x] MM-DEVX-010 Create `.env.example` and env loading policy (包含必填变量、默认值说明、无密钥泄露) <!-- priority:H est:0.5h dep:MM-SCAF-009 owner:BE sp:1 risk:遗漏关键变量 rollback:启动时强校验缺失变量 -->
- [x] MM-DEVX-011 Add Makefile/justfile developer shortcuts (`bootstrap`,`lint`,`test`,`run` 可执行) <!-- priority:M est:0.5h dep:MM-DEVX-001 owner:DevOps sp:1 risk:命令漂移 rollback:统一引用脚本入口 -->
- [ ] MM-DEVX-012 Validate onboarding from clean machine (新机器 30 分钟内完成启动验证) <!-- priority:H est:1h dep:MM-DEVX-011 owner:QA sp:2 risk:文档与脚本不一致 rollback:回归脚本并补充断言 -->

### 5) 核心功能迭代拆解到函数级（模块→子模块→文件→函数→测试→示例）

- 核心里程碑 M1：Domain + Store + Write Path
  - [x] MM-CORE-001 Create crate `memory-core` skeleton (crate 可编译并导出公共模块) <!-- priority:H est:1h dep:MM-SCAF-002 owner:BE sp:2 risk:模块边界不清 rollback:回退到最小模块集 -->
  - [x] MM-CORE-002 Create file `crates/memory-core/src/domain/mod.rs` (导出 Artifact/Episode/Memory/Entity/Relation/Scope 类型) <!-- priority:H est:1h dep:MM-CORE-001 owner:BE sp:2 risk:类型耦合 rollback:拆分子模块 -->
  - [x] MM-CORE-003 Implement struct `Artifact` fields and invariants (含 content_hash 与 source refs 校验) <!-- priority:H est:1h dep:MM-CORE-002 owner:BE sp:2 risk:字段缺失 rollback:以 V2 通用字段补齐 -->
  - [x] MM-CORE-004 Implement struct `Episode` lifecycle enum and transitions (状态变更仅允许 open→idle→closed→archived) <!-- priority:H est:1h dep:MM-CORE-002 owner:BE sp:2 risk:非法状态迁移 rollback:增加状态机守卫 -->
  - [x] MM-CORE-005 Implement struct `Memory` scoring fields and status enum (维护 confidence/importance/stability/freshness) <!-- priority:H est:1h dep:MM-CORE-002 owner:BE sp:2 risk:评分语义混乱 rollback:抽离评分值对象 -->
  - [x] MM-CORE-006 Implement struct `Entity` normalization key logic (提供 `normalized_key()` 函数并可单测) <!-- priority:H est:1h dep:MM-CORE-002 owner:BE sp:2 risk:合并误判 rollback:保留人工确认开关 -->
  - [x] MM-CORE-007 Implement struct `Relation` with evidence requirement checker (无 evidence 拒绝激活) <!-- priority:H est:1h dep:MM-CORE-002 owner:BE sp:2 risk:历史数据兼容 rollback:引入候选态 -->
  - [x] MM-CORE-008 Implement struct `Scope` and hierarchy validator (禁止环形 parent_scope) <!-- priority:H est:1h dep:MM-CORE-002 owner:BE sp:2 risk:树校验性能 rollback:批处理校验 -->
  - [x] MM-CORE-009 Implement trait file `crates/memory-core/src/ports/memory_store.rs` (定义 write/search/fetch_context 接口) <!-- priority:H est:1h dep:MM-CORE-001 owner:BE sp:2 risk:接口未来不兼容 rollback:增加版本化 trait -->
  - [x] MM-CORE-010 Add unit tests for domain invariants in `crates/memory-core/tests/domain_tests.rs` (覆盖 Artifact 去重键、Scope 树、Memory 状态机) <!-- priority:H est:1h dep:MM-CORE-003 owner:QA sp:2 risk:断言不稳 rollback:拆分确定性测试数据 -->

- 核心里程碑 M2：PostgreSQL Driver
  - [x] MM-PG-001 Create crate `memory-store-pg` with SQLx setup (crate 编译并可连接 PG) <!-- priority:H est:1h dep:MM-CORE-009 owner:BE sp:2 risk:连接参数错误 rollback:使用最小 DSN 模板 -->
  - [x] MM-PG-002 Create migration file `migrations/0001_init_scopes.sql` (创建 tenants/scopes/principals 主表成功) <!-- priority:H est:1h dep:MM-DEP-006 owner:DBA sp:2 risk:DDL 锁表 rollback:拆分迁移批次 -->
  - [x] MM-PG-003 Create migration file `migrations/0002_init_content.sql` (创建 episodes/artifacts/memories 表成功) <!-- priority:H est:1h dep:MM-PG-002 owner:DBA sp:2 risk:索引缺失 rollback:补充在线索引迁移 -->
  - [x] MM-PG-004 Implement repository file `src/repo/artifact_repo.rs` function `insert_artifact` (返回持久化 artifact_id 且事务提交) <!-- priority:H est:1h dep:MM-PG-003 owner:BE sp:2 risk:幂等冲突 rollback:按 content_hash upsert -->
  - [x] MM-PG-005 Implement repository file `src/repo/memory_repo.rs` function `insert_memory` (写入 memory 与 version 记录) <!-- priority:H est:1h dep:MM-PG-003 owner:BE sp:2 risk:版本竞争 rollback:乐观锁重试 -->
  - [x] MM-PG-006 Implement function `link_evidence` in `src/repo/evidence_repo.rs` (每条 memory 至少绑定一条 evidence) <!-- priority:H est:1h dep:MM-PG-004 owner:BE sp:2 risk:外键约束失败 rollback:事务内先写 artifact -->
  - [x] MM-PG-007 Implement lexical search query in `src/repo/search_repo.rs` (支持 scope + keyword + time filter) <!-- priority:H est:1h dep:MM-PG-003 owner:BE sp:2 risk:查询慢 rollback:添加 GIN/BTREE 组合索引 -->
  - [x] MM-PG-008 Implement integration tests `tests/pg_store_integration.rs` (覆盖写入/检索/事务回滚) <!-- priority:H est:1.5h dep:MM-PG-007 owner:QA sp:3 risk:环境不稳定 rollback:Testcontainer 隔离 -->

- 核心里程碑 M3：Markdown Driver
  - [x] MM-MD-001 Create crate `memory-store-md` and filesystem abstraction (支持本地目录根路径配置) <!-- priority:H est:1h dep:MM-CORE-009 owner:BE sp:2 risk:路径注入 rollback:路径白名单校验 -->
  - [x] MM-MD-002 Create file renderer `src/render/frontmatter.rs` function `render_frontmatter` (输出字段完整且可解析) <!-- priority:H est:1h dep:MM-MD-001 owner:BE sp:2 risk:YAML 转义错误 rollback:引入序列化库 -->
  - [x] MM-MD-003 Implement writer `src/repo/memory_file_repo.rs` function `write_memory_markdown` (生成 MEMORY.md 条目并落盘) <!-- priority:H est:1h dep:MM-MD-002 owner:BE sp:2 risk:并发写覆盖 rollback:文件锁+临时文件替换 -->
  - [x] MM-MD-004 Implement episode file writer `src/repo/episode_file_repo.rs` (按 YYYY/MM/DD 分层写入 episode 文件) <!-- priority:M est:1h dep:MM-MD-001 owner:BE sp:2 risk:时间分区错误 rollback:统一 UTC 写入 -->
  - [x] MM-MD-005 Implement read-side parser `src/repo/parser.rs` function `parse_memory_markdown` (解析 frontmatter 与 body) <!-- priority:H est:1h dep:MM-MD-003 owner:BE sp:2 risk:格式漂移 rollback:schema 校验并忽略未知字段 -->
  - [x] MM-MD-006 Add unit tests `tests/md_roundtrip.rs` (写入再读取结果一致) <!-- priority:H est:1h dep:MM-MD-005 owner:QA sp:2 risk:换行差异 rollback:规范化换行符 -->

- 核心里程碑 M4：Policy + Distill + Entity/Relation
  - [x] MM-POL-001 Create crate `memory-policy` with policy evaluator trait (可注入 retention/redaction/sync/publish 规则) <!-- priority:H est:1h dep:MM-CORE-001 owner:BE sp:2 risk:策略硬编码 rollback:规则配置化 -->
  - [x] MM-POL-002 Implement function `evaluate_write_policy` (输出 allow/deny/redact 决策) <!-- priority:H est:1h dep:MM-POL-001 owner:BE sp:2 risk:漏判敏感级 rollback:默认拒绝策略 -->
  - [x] MM-EXT-001 Create crate `memory-extract` with distiller pipeline interfaces (定义 summary/entity/relation 抽取端口) <!-- priority:H est:1h dep:MM-CORE-001 owner:BE sp:2 risk:模型耦合 rollback:抽象 provider trait -->
  - [x] MM-EXT-002 Implement function `distill_candidate_memory` in `src/distill.rs` (从 artifact 产出 candidate memory) <!-- priority:H est:1h dep:MM-EXT-001 owner:BE sp:2 risk:幻觉内容 rollback:必须绑定 evidence -->
  - [x] MM-EXT-003 Implement function `extract_entities` in `src/entity.rs` (返回标准化 entity 候选) <!-- priority:H est:1h dep:MM-EXT-001 owner:BE sp:2 risk:实体过拟合 rollback:置信度阈值过滤 -->
  - [x] MM-EXT-004 Implement function `extract_relations` in `src/relation.rs` (输出候选 relation 并带置信度) <!-- priority:H est:1h dep:MM-EXT-003 owner:BE sp:2 risk:关系噪声高 rollback:候选态不入主索引 -->
  - [x] MM-EXT-005 Add unit tests `tests/extract_pipeline_tests.rs` (覆盖空输入、低置信度、跨语言样例) <!-- priority:H est:1h dep:MM-EXT-004 owner:QA sp:2 risk:样本偏差 rollback:扩充基准语料 -->

- 核心里程碑 M5：Kernel 编排与写入链路
  - [x] MM-KER-001 Create crate `memory-kernel` orchestration service (暴露 remember/search/fetch_context 主接口) <!-- priority:H est:1h dep:MM-CORE-009 owner:BE sp:2 risk:依赖反转失败 rollback:按 port-adapter 重新拆分 -->
  - [x] MM-KER-002 Implement function `remember_text` in `src/service/remember.rs` (按 11 步链路完成写入与异步索引触发) <!-- priority:H est:1.5h dep:MM-PG-006 owner:BE sp:3 risk:链路过长失败 rollback:分阶段补偿重试 -->
  - [ ] MM-KER-003 Implement function `remember_media` in `src/service/remember_media.rs` (资产入库后触发后台多模态任务) <!-- priority:H est:1.5h dep:MM-ENV-013 owner:BE sp:3 risk:媒体处理超时 rollback:降级仅存原资产 -->
  - [x] MM-KER-004 Implement function `search_context` in `src/service/search.rs` (并发词法/语义/图谱召回与重排) <!-- priority:H est:1.5h dep:MM-PG-007 owner:BE sp:3 risk:召回不稳定 rollback:固定权重策略 -->
  - [x] MM-KER-005 Implement function `publish_memory` in `src/service/publish.rs` (执行 user→project→team→org 发布链路) <!-- priority:M est:1h dep:MM-POL-002 owner:BE sp:2 risk:权限越界 rollback:强制 policy 审核 -->
  - [x] MM-KER-006 Add integration tests `tests/kernel_flow_tests.rs` (覆盖 remember→search→publish 主路径) <!-- priority:H est:1.5h dep:MM-KER-005 owner:QA sp:3 risk:测试编排复杂 rollback:拆分场景最小化 -->

- 核心里程碑 M6：接入层（MCP/HTTP/CLI）
  - [x] MM-MCP-001 Create crate `memory-mcp` server skeleton (提供 stdio 与 http transport 启动能力) <!-- priority:H est:1h dep:MM-KER-001 owner:BE sp:2 risk:协议细节变化 rollback:封装 transport 适配层 -->
  - [x] MM-MCP-002 Implement tool handler `memory.remember` (输入校验并调用 kernel remember_text) <!-- priority:H est:1h dep:MM-MCP-001 owner:BE sp:2 risk:参数 schema 漂移 rollback:版本化 tool schema -->
  - [x] MM-MCP-003 Implement tool handler `memory.search` (返回结构化 ContextBundle) <!-- priority:H est:1h dep:MM-MCP-002 owner:BE sp:2 risk:返回体过大 rollback:增加 token budget 策略 -->
  - [x] MM-MCP-004 Add MCP integration tests `tests/mcp_tools_tests.rs` (覆盖 remember/search/fetch_context 三工具) <!-- priority:H est:1h dep:MM-MCP-003 owner:QA sp:2 risk:mock 不稳定 rollback:本地环回集成测试 -->
  - [x] MM-HTTP-001 Create crate `memory-http` and router setup (服务可监听并暴露健康检查) <!-- priority:H est:1h dep:MM-KER-001 owner:BE sp:2 risk:中间件顺序错误 rollback:最小路由后逐步叠加 -->
  - [x] MM-HTTP-002 Implement endpoint `POST /memories` handler (写入成功返回 memory_id 与状态) <!-- priority:H est:1h dep:MM-HTTP-001 owner:BE sp:2 risk:鉴权缺失 rollback:统一 auth middleware -->
  - [x] MM-HTTP-003 Implement endpoint `POST /context/search` handler (返回 top_memories 与 evidence_refs) <!-- priority:H est:1h dep:MM-HTTP-002 owner:BE sp:2 risk:查询超时 rollback:增加分页与超时控制 -->
  - [x] MM-HTTP-004 Add integration tests `tests/http_api_tests.rs` (覆盖成功/鉴权失败/参数错误) <!-- priority:H est:1h dep:MM-HTTP-003 owner:QA sp:2 risk:端口占用 rollback:随机端口运行 -->
  - [x] MM-CLI-001 Create crate `memory-cli` command skeleton (支持 `remember`,`search`,`serve` 子命令) <!-- priority:M est:1h dep:MM-KER-001 owner:BE sp:2 risk:参数过多 rollback:简化命令结构 -->
  - [x] MM-CLI-002 Implement command `memory-cli remember` (可从 stdin 或文件写入记忆) <!-- priority:M est:1h dep:MM-CLI-001 owner:BE sp:2 risk:编码格式异常 rollback:强制 UTF-8 检测 -->
  - [x] MM-CLI-003 Implement command `memory-cli search` (输出结构化 JSON 与简报模式) <!-- priority:M est:1h dep:MM-CLI-002 owner:BE sp:2 risk:输出不兼容 rollback:版本化输出 schema -->

- 核心里程碑 M7：同步与后台任务
  - [x] MM-SYNC-001 Create crate `memory-sync` with oplog interfaces (定义 append/pull/merge API) <!-- priority:H est:1h dep:MM-CORE-001 owner:BE sp:2 risk:一致性边界不清 rollback:先支持单向同步 -->
  - [x] MM-SYNC-002 Implement function `append_oplog_entry` in `src/oplog.rs` (每次写入生成可追溯 op 记录) <!-- priority:H est:1h dep:MM-SYNC-001 owner:BE sp:2 risk:顺序错乱 rollback:按时间序+版本向量 -->
  - [x] MM-SYNC-003 Implement function `merge_ops` in `src/merge.rs` (冲突对象输出 merged/conflicted 结果) <!-- priority:H est:1.5h dep:MM-SYNC-002 owner:BE sp:3 risk:误合并 rollback:保留冲突副本 -->
  - [ ] MM-SYNC-004 Create crate `memory-worker` and queue consumer loop (后台任务可拉取并执行) <!-- priority:H est:1h dep:MM-KER-003 owner:BE sp:2 risk:任务丢失 rollback:引入死信队列 -->
  - [ ] MM-SYNC-005 Implement job `projection_refresh_job` (对象变更后刷新 markdown 与 agent 投影) <!-- priority:M est:1h dep:MM-SYNC-004 owner:BE sp:2 risk:投影滞后 rollback:定时全量重建 -->
  - [x] MM-SYNC-006 Add integration tests `tests/sync_merge_tests.rs` (覆盖冲突、重试、死信回放) <!-- priority:H est:1.5h dep:MM-SYNC-003 owner:QA sp:3 risk:场景复杂 rollback:拆分为单对象场景 -->

- 核心里程碑 M8：示例与开发文档验收
  - [ ] MM-EXM-001 Add example `examples/remember_text.rs` (可独立运行写入示例) <!-- priority:M est:0.5h dep:MM-KER-002 owner:TechWriter sp:1 risk:示例过期 rollback:CI 定时编译示例 -->
  - [ ] MM-EXM-002 Add example `examples/search_context.rs` (可演示检索与重排输出) <!-- priority:M est:0.5h dep:MM-KER-004 owner:TechWriter sp:1 risk:示例依赖外部服务 rollback:提供 mock 模式 -->
  - [ ] MM-EXM-003 Add API usage snippets for MCP/HTTP/CLI (文档示例可复制运行) <!-- priority:M est:1h dep:MM-MCP-004 owner:TechWriter sp:2 risk:示例不一致 rollback:自动从测试生成片段 -->

### 6) 自动化测试策略

- [ ] MM-TST-001 Define test pyramid and coverage thresholds (单元/集成/e2e 覆盖率目标明确并入 CI) <!-- priority:H est:0.5h dep:MM-SCAF-010 owner:QA sp:1 risk:阈值过高 rollback:阶段性阈值 -->
- [ ] MM-TST-002 Add static analysis stage (fmt/clippy/security/license 全量执行) <!-- priority:H est:1h dep:MM-DEVX-001 owner:QA sp:2 risk:执行耗时长 rollback:并行拆分 job -->
- [ ] MM-TST-003 Build unit test matrix by crate (每个 crate 至少 1 组核心单测) <!-- priority:H est:1h dep:MM-CORE-010 owner:QA sp:2 risk:盲区遗漏 rollback:按风险补齐 -->
- [ ] MM-TST-004 Build integration test matrix by critical flow (写入/检索/同步/发布主链路全覆盖) <!-- priority:H est:1h dep:MM-KER-006 owner:QA sp:2 risk:环境不稳定 rollback:容器化隔离 -->
- [ ] MM-TST-005 Build end-to-end tests with real transports (MCP+HTTP+CLI 全链路验收) <!-- priority:H est:1.5h dep:MM-HTTP-004 owner:QA sp:3 risk:外部依赖抖动 rollback:录制回放 fixture -->
- [ ] MM-TST-006 Add performance benchmark suite (定义 P50/P95 延迟与吞吐基准) <!-- priority:M est:1h dep:MM-KER-004 owner:SRE sp:2 risk:噪音大 rollback:固定硬件基线 -->
- [ ] MM-TST-007 Add security test suite (鉴权、注入、越权、敏感信息泄露检查) <!-- priority:H est:1h dep:MM-POL-002 owner:Sec sp:2 risk:误报多 rollback:分级策略处理 -->
- [ ] MM-TST-008 Add compatibility tests (macOS/Linux、PG 版本矩阵、文件系统差异) <!-- priority:M est:1h dep:MM-ENV-015 owner:QA sp:2 risk:矩阵过大 rollback:核心组合优先 -->
- [ ] MM-TST-009 Add regression test gate for bugfix labels (每个缺陷关联回归用例) <!-- priority:M est:0.5h dep:MM-TST-004 owner:QA sp:1 risk:维护成本高 rollback:自动标记脚本 -->
- [ ] MM-TST-010 Add smoke tests for release artifacts (发布包启动与健康检查通过) <!-- priority:H est:0.5h dep:MM-CI-010 owner:QA sp:1 risk:检查过浅 rollback:扩展关键接口探针 -->
- [ ] MM-TST-011 Define blue-green validation tests (双环境对比关键指标无回归) <!-- priority:M est:1h dep:MM-REL-008 owner:SRE sp:2 risk:成本高 rollback:降级为单关键流验证 -->
- [ ] MM-TST-012 Define canary analysis tests (小流量阶段错误率/延迟阈值自动判定) <!-- priority:M est:1h dep:MM-REL-009 owner:SRE sp:2 risk:阈值误设 rollback:手动审批兜底 -->

### 7) 构建与部署流水线

- [ ] MM-CI-001 Create CI workflow stages for lint/test/build/security (PR 流水线并行执行且可视化) <!-- priority:H est:1h dep:MM-SCAF-011 owner:DevOps sp:2 risk:队列拥堵 rollback:缓存与并行优化 -->
- [ ] MM-CI-002 Implement multi-stage Docker build (builder/runtime 分层，最终镜像可运行) <!-- priority:H est:1h dep:MM-DEVX-006 owner:DevOps sp:2 risk:运行时缺库 rollback:补齐最小运行依赖 -->
- [ ] MM-CI-003 Enable dependency and cargo target caching (构建时间显著降低且命中率可观测) <!-- priority:M est:1h dep:MM-CI-001 owner:DevOps sp:2 risk:缓存污染 rollback:按 key 维度隔离 -->
- [ ] MM-CI-004 Enable binary compression and strip policy (发布产物体积达标且可执行) <!-- priority:M est:0.5h dep:MM-CI-002 owner:DevOps sp:1 risk:调试符号丢失 rollback:保留符号副本 -->
- [ ] MM-CI-005 Enable artifact signing for binaries/images (签名可验证且密钥托管合规) <!-- priority:H est:1h dep:MM-CI-004 owner:Sec sp:2 risk:密钥泄露 rollback:吊销并轮换密钥 -->
- [ ] MM-CI-006 Integrate container image scanning gate (高危漏洞阻断发布) <!-- priority:H est:0.5h dep:MM-CI-002 owner:Sec sp:1 risk:扫描误报 rollback:白名单 + SLA 复核 -->
- [ ] MM-CI-007 Create Helm chart skeleton (`Chart.yaml`,`values.yaml`,`templates` 可安装) <!-- priority:M est:1h dep:MM-CI-002 owner:DevOps sp:2 risk:模板参数不全 rollback:补齐默认值 -->
- [ ] MM-CI-008 Define IaC modules for network/storage/compute (基础设施可重复创建销毁) <!-- priority:M est:1.5h dep:MM-ENV-011 owner:DevOps sp:3 risk:资源漂移 rollback:state 锁定与导入 -->
- [ ] MM-CI-009 Implement GitOps deployment manifests (环境配置变更可通过 PR 审核生效) <!-- priority:M est:1h dep:MM-CI-007 owner:DevOps sp:2 risk:配置漂移 rollback:强制声明式回收 -->
- [ ] MM-CI-010 Create release workflow with semantic version tags (打 tag 自动生成产物与发布记录) <!-- priority:H est:1h dep:MM-CI-001 owner:DevOps sp:2 risk:版本号冲突 rollback:预发布通道校验 -->
- [ ] MM-REL-001 Define rollback tag strategy (每次发布关联可回滚镜像与数据库迁移版本) <!-- priority:H est:0.5h dep:MM-CI-010 owner:DevOps sp:1 risk:回滚点缺失 rollback:强制发布前快照 -->
- [ ] MM-REL-002 Implement database migration rollback scripts (关键迁移具备 down 或补偿脚本) <!-- priority:H est:1h dep:MM-PG-003 owner:DBA sp:2 risk:数据丢失 rollback:备份恢复方案 -->
- [ ] MM-REL-003 Automate release note generation (发布记录包含变更摘要与风险项) <!-- priority:M est:0.5h dep:MM-CI-010 owner:PM sp:1 risk:信息不完整 rollback:手动补录流程 -->
- [ ] MM-REL-004 Validate deployment dry-run in staging (staging 全流程部署成功并通过 smoke) <!-- priority:H est:1h dep:MM-TST-010 owner:SRE sp:2 risk:环境差异 rollback:基础设施对齐 -->
- [ ] MM-REL-005 Implement production approval gates (生产发布需安全与SRE双审批) <!-- priority:M est:0.5h dep:MM-REL-004 owner:PM sp:1 risk:审批延迟 rollback:紧急发布绿色通道 -->
- [ ] MM-REL-006 Implement automated rollback action (告警触发后可一键回滚至上个稳定版本) <!-- priority:H est:1h dep:MM-REL-001 owner:SRE sp:2 risk:回滚失败 rollback:手工 runbook 回滚 -->
- [ ] MM-REL-007 Record deployment metadata to release registry (记录版本、环境、执行人、结果) <!-- priority:M est:0.5h dep:MM-REL-004 owner:DevOps sp:1 risk:元数据缺失 rollback:从 CI 日志补采 -->
- [ ] MM-REL-008 Define blue-green rollout workflow (绿环境验证通过后切换流量) <!-- priority:M est:1h dep:MM-REL-004 owner:SRE sp:2 risk:流量切换抖动 rollback:DNS/网关快速切回 -->
- [ ] MM-REL-009 Define canary rollout workflow (按百分比递增并结合指标自动推进) <!-- priority:M est:1h dep:MM-REL-004 owner:SRE sp:2 risk:样本不足 rollback:固定小流量人工观测 -->

### 8) 监控与可观测性埋点

- [x] MM-OBS-001 Define structured logging schema (日志包含 trace_id/span_id/scope/task_id) <!-- priority:H est:0.5h dep:MM-DEP-005 owner:SRE sp:1 risk:字段不统一 rollback:集中日志中间件 -->
- [x] MM-OBS-002 Instrument write-path logs in kernel (remember 链路关键节点可观测) <!-- priority:H est:1h dep:MM-KER-002 owner:BE sp:2 risk:日志噪音高 rollback:级别与采样控制 -->
- [x] MM-OBS-003 Instrument retrieval metrics (QPS、命中率、P95、重排耗时可采集) <!-- priority:H est:1h dep:MM-KER-004 owner:SRE sp:2 risk:指标口径不一致 rollback:统一指标词典 -->
- [ ] MM-OBS-004 Instrument sync pipeline metrics (队列长度、重试次数、死信数量可观测) <!-- priority:H est:1h dep:MM-SYNC-004 owner:SRE sp:2 risk:遗漏边缘状态 rollback:增加状态机计数器 -->
- [ ] MM-OBS-005 Enable distributed tracing across HTTP/MCP/worker (跨组件请求可串联) <!-- priority:H est:1h dep:MM-OBS-001 owner:SRE sp:2 risk:采样过低 rollback:关键路径强制采样 -->
- [x] MM-OBS-006 Add health endpoints for app/deps (`/healthz`、`/readyz`、`/livez` 可用) <!-- priority:H est:0.5h dep:MM-HTTP-001 owner:BE sp:1 risk:健康检查失真 rollback:分级健康探针 -->
- [ ] MM-OBS-007 Define alert rules for SLI breaches (错误率/延迟/队列积压触发告警) <!-- priority:H est:1h dep:MM-OBS-003 owner:SRE sp:2 risk:告警风暴 rollback:抑制与聚合策略 -->
- [ ] MM-OBS-008 Define SLOs for write/search/sync services (SLO 文档化并与监控绑定) <!-- priority:M est:1h dep:MM-OBS-007 owner:SRE sp:2 risk:目标不现实 rollback:阶段性目标迭代 -->
- [ ] MM-OBS-009 Define error budget policy and burn alerts (预算消耗可视并驱动发布决策) <!-- priority:M est:0.5h dep:MM-OBS-008 owner:SRE sp:1 risk:预算口径争议 rollback:按服务拆分预算 -->
- [ ] MM-OBS-010 Build observability dashboard pack (提供业务+系统+安全三类看板) <!-- priority:M est:1h dep:MM-OBS-007 owner:SRE sp:2 risk:看板维护成本高 rollback:保留核心 dashboard -->

### 9) 文档体系

- [ ] MM-DOC-001 Produce architecture diagram from implemented modules (图与代码目录一一对应) <!-- priority:H est:1h dep:MM-KER-001 owner:TechWriter sp:2 risk:图文脱节 rollback:自动从模块生成清单 -->
- [ ] MM-DOC-002 Produce sequence diagram for remember/search flows (覆盖成功、失败、补偿分支) <!-- priority:H est:1h dep:MM-KER-006 owner:TechWriter sp:2 risk:遗漏异常分支 rollback:从测试用例反推 -->
- [ ] MM-DOC-003 Produce ER diagram for PostgreSQL table families (主外键与索引策略清晰) <!-- priority:H est:1h dep:MM-PG-003 owner:DBA sp:2 risk:迁移后失效 rollback:CI 自动校验 ER 导出 -->
- [ ] MM-DOC-004 Publish API reference for HTTP endpoints (请求/响应/错误码可检索) <!-- priority:H est:1h dep:MM-HTTP-004 owner:TechWriter sp:2 risk:接口变更频繁 rollback:从 OpenAPI 自动生成 -->
- [ ] MM-DOC-005 Publish MCP tool reference (工具入参、出参、示例覆盖 remember/search/fetch) <!-- priority:H est:1h dep:MM-MCP-004 owner:TechWriter sp:2 risk:工具版本漂移 rollback:从 schema 自动导出 -->
- [ ] MM-DOC-006 Publish operations manual for local/cloud/hybrid modes (包含启动、扩缩容、备份恢复) <!-- priority:H est:1h dep:MM-REL-004 owner:SRE sp:2 risk:步骤缺失 rollback:Runbook 演练补齐 -->
- [ ] MM-DOC-007 Publish incident runbook and rollback playbook (常见故障具备诊断与处置步骤) <!-- priority:H est:1h dep:MM-REL-006 owner:SRE sp:2 risk:实战不可用 rollback:定期演练更新 -->
- [ ] MM-DOC-008 Publish FAQ for developers/operators (覆盖安装、配置、性能、安全高频问题) <!-- priority:M est:0.5h dep:MM-DOC-006 owner:TechWriter sp:1 risk:长期失效 rollback:双周维护窗口 -->
- [ ] MM-DOC-009 Publish version migration guide (V1→V2 schema/config/data 迁移步骤明确) <!-- priority:H est:1h dep:MM-REL-002 owner:DBA sp:2 risk:迁移中断 rollback:备份+分批迁移策略 -->
- [ ] MM-DOC-010 Publish troubleshooting index by symptom (按错误码/日志关键词快速定位方案) <!-- priority:M est:0.5h dep:MM-DOC-007 owner:Support sp:1 risk:搜索命中低 rollback:增加标签体系 -->

## 扁平原子任务模板（复用）

- [ ] TASK-ID verb description (验收标准) <!-- priority:H/M/L est:0.5h dep:TASK-001 owner:xxx sp:1 risk:xxx rollback:xxx -->
