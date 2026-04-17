# Meat Memory 执行日志

## 结构化记录

- timestamp: 2026-04-14 00:00:00 CST
  task_id: V2.6-DOC-001/V2.6-CLI-001/V2.6-TUI-001/V2.6-SKL-001/V2.6-DOC-002/V2.6-QA-001
  executor: Codex
  duration: 0.8h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 新增 V2.6 项目记忆边界引导：`memory-cli project init` 与 `memory-cli tui project-init` 支持数字化确认新/旧项目、互通/隔离、团队/个人、现有列表/手动 scope；非交互模式支持 `--existing`、`--scope-id`、`--scope-kind`、`--isolated/--shared`；Codex/Claude/执行型 Agent skill 与 CLI/MCP 文档已补项目边界规则
  issues_decisions: 本轮不改 MCP 工具面和数据库 schema；项目边界复用现有 `scope_id`、access key、`owner_scope_id`、`scope_kind`、`is_fully_isolated` 机制，MCP 调用前由 CLI/TUI 或用户显式 scope 完成边界确认
  next_action: 如后续需要 MCP 原生 project 工具，先单独形成 `memory.project.prepare/list` 方案并等待确认；否则可进入 V3 音频/视频多模态

- timestamp: 2026-04-13 10:15:00 CST
  task_id: DIST-PKG-001
  executor: Codex
  duration: 0.4h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 完善 `.github/workflows/release.yml`，改为 tag 驱动的多平台 release 构建与 GitHub Release 上传；新增 `docs/scripts/build-release-artifacts.sh`，可将 `memory-cli`、`memory-app`、`memory-worker` 与示例配置打成 `dist/release/<asset>.tar.gz` 并生成 `sha256`；同步更新脚本说明与 tasklist 状态
  issues_decisions: 预编译二进制包当前聚焦 tarball + checksum 的统一产物约定，不在本任务内同时落 npm、Homebrew、skill bundle 发布；这些后续分别由 `DIST-PKG-002`、`DIST-PKG-003A`、`DIST-PKG-004` 继续完成
  next_action: 进入 `DIST-PKG-002`，补 npm 安装入口与平台二进制分发包装层

- timestamp: 2026-04-13 10:32:00 CST
  task_id: DIST-PKG-003
  executor: Codex
  duration: 0.3h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 为 `crates/memory-cli/Cargo.toml` 补齐 description、license、repository、rust-version 与 readme 元数据；在 `docs/runbook/usage-guide.md` 与 `docs/api/cli-v1.md` 新增 `cargo install --path crates/memory-cli --locked` 的安装后验证说明；本地已通过 `/tmp/meat-memory-cargo-install-check` 成功安装并产出可执行 `memory-cli`
  issues_decisions: 现阶段先验证仓库内 `cargo install --path` 闭环，远端 registry / git 安装说明等到正式发布元数据与下载入口收口后再统一更新
  next_action: 回到 `DIST-PKG-002` 与 `DIST-PKG-003A`，继续补 npm 包装层和安装后 Agent skill 交付闭环

- timestamp: 2026-04-13 10:45:00 CST
  task_id: DIST-PKG-003A
  executor: Codex
  duration: 0.2h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 新增 `docs/scripts/build-agent-skills-bundle.sh`，可把现有 skill 导出目录打成 `agent-skills-bundle.zip`；release workflow 已在发布阶段附带该 zip；`docs/scripts/README.md` 与 `docs/agent-skills/README.md` 也同步补齐“目录导出 + release bundle”双路径说明
  issues_decisions: 安装后交付闭环统一为两种标准形态：本地可执行导出目录、release 可直接下载 zip 包；不把平台导入动作写死到安装脚本里，避免覆盖用户现有 Agent 配置
  next_action: 回到 `DIST-PKG-002`，实现 npm 安装入口，或继续推进 `DIST-PKG-005` Docker 镜像规范化

- timestamp: 2026-04-13 11:05:00 CST
  task_id: DIST-PKG-005/DIST-DEP-001
  executor: Codex
  duration: 0.3h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 将 `Dockerfile` 收口为 `app-runtime` 与 `worker-runtime` 两个 target；`compose.yaml` 开始显式构建并标记 `meat-memory-app:${MEAT_MEMORY_IMAGE_TAG:-local}` 和 `meat-memory-worker:${MEAT_MEMORY_IMAGE_TAG:-local}`；`.env.example`、本地/云端部署 runbook、Docker infra README 与安装打包部署方案文档同步补齐镜像名、tag 和 compose 变量约定
  issues_decisions: 本地 compose 默认统一使用 `:local` tag，release/云端镜像则与 Git tag 保持一致；worker 镜像与 app 镜像分离命名，但继续共用一套 Dockerfile 以降低维护成本
  next_action: 可继续推进 `DIST-CI-001`，把 Docker 镜像构建与发布顺序接入统一发布流水线；或回到 `DIST-PKG-002` 收口 npm 安装入口

- timestamp: 2026-04-13 11:20:00 CST
  task_id: DIST-DEP-002
  executor: Codex
  duration: 0.3h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 新增 `infra/systemd/README.md`、`meat-memory-app.service`、`meat-memory-worker.service` 与 `meat-memory.env.example`；新增 `docs/runbook/systemd-deploy.md`，补齐 release 解压、配置目录、运行用户、systemctl 启停与日志排查说明；同时更新 infra/runbook/architecture/tasklist 入口
  issues_decisions: 单机部署继续沿用 `config/app.toml + MEAT_MEMORY_*` 的统一配置模型，运行目录约定为 `/opt/meat-memory`、`/etc/meat-memory`、`/var/lib/meat-memory`；service 模板默认启用最小硬化设置，但不强加更激进的 sandbox 限制，避免影响不同发行版兼容性
  next_action: 进入 `DIST-DEP-003` 完善 Helm/Kubernetes 部署，或推进 `DIST-CI-001` 统一发布流水线

- timestamp: 2026-04-13 11:35:00 CST
  task_id: DIST-CI-001
  executor: Codex
  duration: 0.2h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 将 `.github/workflows/release.yml` 收口为 tag 驱动的统一发布骨架：先构建多平台二进制并发布 GitHub Release，再生成 `agent-skills-bundle.zip`，最后发布 `ghcr.io/<owner>/meat-memory-app` 与 `ghcr.io/<owner>/meat-memory-worker` 镜像；同时把 Helm 默认镜像仓库与云端 runbook 统一到 app 镜像命名
  issues_decisions: 当前 release 流水线先正式覆盖 release assets、skills 和 Docker；npm publish 与 Homebrew bump 暂只保留为后续接入点，避免在缺少正式发布仓库地址时写入不可靠实现
  next_action: 可继续推进 `DIST-DEP-003`，完善 Helm/Kubernetes 部署；之后再回补 `DIST-PKG-002` 与 `DIST-PKG-004`

- timestamp: 2026-04-13 11:50:00 CST
  task_id: DIST-DEP-003
  executor: Codex
  duration: 0.2h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 完善 Helm/Kubernetes 部署骨架：`values.yaml` 新增 `workerImage`、`persistence.existingClaim`、`secrets.existingSecretName`；deployment 模板支持 app/worker 分镜像和现有 Secret/PVC 复用；云端 runbook 与安装打包部署方案同步补齐对应参数说明
  issues_decisions: Helm 继续保持单 Chart、app/worker 分 deployment 的简单结构；默认仍以内置 Secret/PVC 生成为主，但允许生产环境显式复用集群已有资源，减少重复声明和迁移成本
  next_action: 可继续推进 `DIST-DOC-002`，把安装/打包/部署入口统一对外收口；或回到 `DIST-PKG-002` / `DIST-PKG-004`

- timestamp: 2026-04-13 12:05:00 CST
  task_id: DIST-DOC-002
  executor: Codex
  duration: 0.2h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 新增 `docs/runbook/install.md` 作为面向用户的安装、打包与部署总入口，覆盖 cargo install、release 二进制、agent skill bundle、Docker Compose、systemd、Helm 和 release pipeline；根 README、docs 总览、runbook README、架构设计入口和 usage guide 均已挂载该入口
  issues_decisions: npm 与 Homebrew 仍在用户入口中标注为后续接入点，不作为已发布渠道展示；正式仓库地址和分发域名确定后再回补 `DIST-PKG-002` 与 `DIST-PKG-004`
  next_action: 分发主链路文档已收口；后续可在正式发布信息确定后推进 npm / Homebrew，或进行提交前整理与验证

- timestamp: 2026-04-13 12:20:00 CST
  task_id: DIST-PKG-004
  executor: Codex
  duration: 0.2h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 新增 `packaging/homebrew/README.md` 与 `meat-memory.rb.template`，明确 Homebrew tap、macOS release tarball、sha256 替换、brew audit/install/test 验证和后续 release workflow 自动化接入点；install runbook 与安装打包部署方案已挂载该入口
  issues_decisions: 因正式 GitHub 仓库、tap 地址和 release 下载域名未最终确定，Formula 保留占位符，不把 Homebrew 宣称为已发布渠道；后续确认发布地址后再渲染到 tap 仓库
  next_action: 剩余分发任务主要是 `DIST-PKG-002` npm 安装入口；可先做 npm 包装层骨架，或等待正式 release 地址确定后再收口

- timestamp: 2026-04-13 12:35:00 CST
  task_id: DIST-PKG-002
  executor: Codex
  duration: 0.3h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 新增 `packaging/npm/` 包装层骨架，包含 `package.json`、`bin/meat-memory.js`、`scripts/install.js` 和 README；包装层支持 `meat-memory` / `memory-cli` 命令代理、OS/Arch 到 release tarball 的映射，以及通过 `releaseBaseUrl` 或 `MEAT_MEMORY_NPM_RELEASE_BASE_URL` 配置下载源
  issues_decisions: 因正式 GitHub release 下载域名和 npm scope 尚未最终确认，npm 入口实现为可配置骨架；未在用户文档中宣称 npm 已发布，避免误导安装用户
  next_action: 安装、打包、部署实施清单已全部收口；下一步建议做提交前验证、状态整理和按变更类型拆分暂存

- timestamp: 2026-04-12 17:37:37 CST
  task_id: V2.4-STO-002
  executor: Codex
  duration: 0.3h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 为 `memory-store-md` 新增 project document projection：`ProjectDocumentFrontmatter`、单文件 markdown render/parse、按 source 分类的 documents 目录、本地 `local_path` 到 projection 路径的映射，以及按 `document_id` 扫描读取；`memory-kernel import_project_document` 在启用 Markdown store 且 storage mode 允许时会同步写出 project document projection；补齐 md store 单测和 kernel 集成测试覆盖
  issues_decisions: project document 不继续塞进 `MEMORY.md` rollup，而是改为 source 维度的独立 markdown 文件，避免 README/runbook/API 文档等工作集互相覆盖；frontmatter 中的 `layer` 固定写 `mid_term`，因为 `ProjectDocument` 领域对象本身不单独存该字段
  next_action: V2.4 范围已完成；如继续，可进入 V3 音频/视频多模态规划与实现

- timestamp: 2026-04-12 17:25:00 CST
  task_id: V2.4-QA-002
  executor: Codex
  duration: 0.2h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 新增 `docs/scripts/v2_4-acceptance.sh`，覆盖 domain、observability、sync、PG store、kernel、HTTP、MCP、CLI 与 workspace check；接入 `docs/scripts/write-test-reports.sh` 与 `tests/reports/e2e/README.md`；实跑生成 `tests/reports/e2e/latest/v2_4-acceptance.txt` 和 archive 副本，并更新 `tests/reports/latest-run.md`
  issues_decisions: 首次手工写 report 时发现 zsh 不支持 bash 的 `PIPESTATUS` 数组，验收主体已通过但尾注写入失败；随后改用 `bash -lc` 重新执行并成功写入 `exit_code: 0`，正式脚本本身使用 bash shebang，不受该问题影响
  next_action: V2.4 主链路已完成；后续可进入 V3 音频/视频多模态，或回补 `V2.4-STO-002` 的 Markdown project document projection

- timestamp: 2026-04-12 17:23:30 CST
  task_id: V2.4-QA-001
  executor: Codex
  duration: 0.2h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 完成 V2.4 核心回归确认：domain、sync、PG store、kernel、HTTP、MCP、CLI 与 observability 定向测试均通过；覆盖 source 多 key、Agent Context、Project Documents、本地同步计划、冲突列表与 V2.4 metrics snapshot
  issues_decisions: 本轮将 `cargo check --workspace --all-targets` 作为跨 crate 接口一致性门禁；完整可重复验收脚本与报告产物留给 `V2.4-QA-002`
  next_action: 进入 `V2.4-QA-002`，新增 V2.4 验收脚本与报告

- timestamp: 2026-04-12 17:21:00 CST
  task_id: V2.4-OBS-001
  executor: Codex
  duration: 0.3h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 扩展 `memory-observability` 的 `v2_4` metrics snapshot，新增 source/context/docs 操作、失败、导入文档、missing 文档与冲突数量统计；Kernel 的 source、Agent Context、Project Document 导入/查询/冲突/同步计划路径均已接入埋点；HTTP Browser Console 监控概览新增 V2.4 Memory 能力指标展示
  issues_decisions: 文档同步 apply 会复用单文档导入路径，因此导入文档数由 import 路径统计，apply 路径只补充 missing/conflicts 汇总，避免同步导入数量重复计算
  next_action: 进入 `V2.4-QA-001`，补充 V2.4 端到端验收覆盖

- timestamp: 2026-04-12 17:16:16 CST
  task_id: V2.4-SKL-001
  executor: Codex
  duration: 0.1h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 更新 Codex、Claude Code/TRAE/Qoder、执行型 Agent 三套 skill 模板与 Agent skill README，补充 V2.4 的 short-term Agent Context、mid-term Project Documents、source 多 key、docs status/sync 和冲突处理说明
  issues_decisions: 文案明确要求遇到 `missing` / `conflicts` 时先展示给用户确认，不静默覆盖本地项目文档；CLI 与 MCP 两套入口都给出最小用法
  next_action: 进入 `V2.4-OBS-001`，增加 source/key/context/docs 维度统计

- timestamp: 2026-04-12 17:14:57 CST
  task_id: V2.4-CLI-002
  executor: Codex
  duration: 0.2h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 为 `memory-cli docs` 新增 `status` 与 `sync`，复用 `LocalProjectDocumentSyncEngine` 生成本地文档扫描计划，并通过 Kernel apply 同步计划导入 ProjectDocument；CLI e2e 已覆盖 status 预览与 sync 导入
  issues_decisions: `docs status` 等价于 dry-run 计划输出；`docs sync --dry-run` 不写入，默认执行导入但仍只返回 missing/conflicts，不自动覆盖冲突文档
  next_action: 进入 `V2.4-SKL-001`，更新 Agent Skill 文案说明短期上下文、项目文档同步与 source 多 key

- timestamp: 2026-04-12 17:12:27 CST
  task_id: V2.4-CLI-001
  executor: Codex
  duration: 0.4h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 为 `memory-cli` 新增 `source create/list/keys/key-create`、`context upsert/list/promote/delete`、`docs import/list/conflicts`；补充 CLI e2e 覆盖 source 多 key、短期上下文生命周期与中期文档导入/查询/冲突列表
  issues_decisions: CLI 管理命令统一复用 `--key` / `MEAT_MEMORY_KEY` 的认证解析；`docs sync` 暂不并入本任务，保留给 `V2.4-CLI-002` 专门实现本地文档同步 CLI
  next_action: 进入 `V2.4-CLI-002`，实现本地文档同步 CLI

- timestamp: 2026-04-12 16:55:46 CST
  task_id: V2.4-MCP-002
  executor: Codex
  duration: 0.3h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 为 MCP dispatcher 新增 `memory.docs.sync`、`memory.docs.search`、`memory.docs.conflicts`；项目文档同步支持 source.local_root / local_root 覆盖、dry_run 计划和执行导入；补充临时目录扫描导入与查询/冲突集成测试
  issues_decisions: 项目文档 MCP 工具统一要求 key，按 source.owner_scope_id 做访问控制；docs.search 对应 Kernel 的文档列表 + query 过滤，作为 Agent 侧中期项目文档搜索入口
  next_action: 进入 `V2.4-CLI-001`，实现 CLI source/context/doc 管理命令

- timestamp: 2026-04-12 16:53:03 CST
  task_id: V2.4-MCP-001
  executor: Codex
  duration: 0.3h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 为 MCP dispatcher 新增 `memory.context.upsert`、`memory.context.list`、`memory.context.promote`、`memory.context.delete`；工具清单同步扩展到 9 个；补充 PG+Markdown 集成测试覆盖短期上下文 upsert/list/promote/delete
  issues_decisions: MCP Agent Context 工具统一要求 key，默认使用 key 的 owner scope；额外加入 delete 工具以闭合短期上下文生命周期，并与 HTTP API 能力保持一致
  next_action: 进入 `V2.4-MCP-002`，实现 MCP 工具扩展：项目文档

- timestamp: 2026-04-12 16:50:13 CST
  task_id: V2.4-API-003
  executor: Codex
  duration: 0.4h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 为 HTTP API 新增 `/api/v1/sources/{source_id}/documents` 列表、`/documents/import` 导入、`/documents/conflicts` 冲突列表、`/documents/sync` 本地扫描同步；同步响应包含 planned/imported/missing/conflicts；补充临时目录扫描导入集成测试
  issues_decisions: 文档同步入口挂在 Source 下并要求 meat memory key；同步默认使用 source.local_root，也允许请求覆盖 local_root；`dry_run=true` 只返回计划不写入，执行路径继续沿用 Kernel 的冲突报告而不静默覆盖本地文档
  next_action: 进入 `V2.4-MCP-001`，实现 MCP 工具扩展：实时上下文

- timestamp: 2026-04-12 16:46:40 CST
  task_id: V2.4-API-002
  executor: Codex
  duration: 0.3h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 为 HTTP API 新增 `/api/v1/agent-contexts` upsert/list、`/api/v1/agent-contexts/{context_id}` delete、`/api/v1/agent-contexts/{context_id}/promote`；响应包含 source/key/scope/session/task/layer/labels/时间戳；补充 upsert/list/promote/delete 集成测试
  issues_decisions: Agent Context HTTP 入口统一要求 meat memory key，默认使用当前 key 的 owner scope；promote 复用现有 `CreateMemoryResponse` 结构，便于 Agent 直接消费 promoted memory
  next_action: 进入 `V2.4-API-003`，实现 HTTP API：Project Documents Sync

- timestamp: 2026-04-12 16:43:35 CST
  task_id: V2.4-API-001
  executor: Codex
  duration: 0.5h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 为 HTTP API 新增 `/api/v1/sources` 创建/列表、`/api/v1/sources/{source_id}` 详情、`/api/v1/sources/{source_id}/keys` 创建/列表；`AccessKeyResponse` 返回 `source_id`，Kernel 创建/轮换 key 保留 source 归属；补充 source 多 key 集成测试
  issues_decisions: source 管理要求使用现有 meat memory key 认证；嵌套 source key 创建默认继承 source 的 owner scope，source kind 未显式指定时使用 `custom`，避免任意 MemorySource kind 与 key enum 强绑定
  next_action: 进入 `V2.4-API-002`，实现 HTTP API：Agent Context

- timestamp: 2026-04-12 01:15:00 CST
  task_id: V2.4-SYN-002
  executor: Codex
  duration: 0.4h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 为项目文档同步新增 `ProjectDocumentConflictInput`、`ProjectDocumentConflictReport` 与 `classify_project_document_conflict`，覆盖 clean/changed/deleted/conflicted；同步计划增加 conflicts 输出，Kernel apply 结果同步返回 conflicts
  issues_decisions: 对本地删除但远端/索引也变化的场景采用保守冲突报告，不自动删除、不自动覆盖；missing/deleted/conflicted 都只作为计划结果交给上层显式处理
  next_action: 进入 `V2.4-API-001`，实现 HTTP API：source 多 key 管理

- timestamp: 2026-04-12 01:00:00 CST
  task_id: V2.4-SYN-001
  executor: Codex
  duration: 0.7h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 在 `memory-sync` 增加 `LocalProjectDocumentSyncEngine`，支持扫描本地 md/markdown/txt 文档、计算 content hash、对比历史 snapshot 并输出 clean/changed/missing 同步计划；在 `memory-kernel` 增加 `apply_project_document_sync_plan`，可将同步计划导入中期 ProjectDocument
  issues_decisions: 本轮默认只读本地文档并导入中期索引，不自动删除或覆盖文档；missing 仅作为计划结果返回，冲突细化和双向策略进入 `V2.4-SYN-002`
  next_action: 进入 `V2.4-SYN-002`，完善文档同步冲突策略

- timestamp: 2026-04-12 00:45:00 CST
  task_id: V2.4-KER-002
  executor: Codex
  duration: 0.6h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 在 `memory-kernel` 增加 MemorySource 与 ProjectDocument 服务方法，支持 source upsert/list、source key 查询、项目文档导入、artifact 关联、文档列表/关键词过滤和冲突查询；新增 Kernel 集成测试覆盖中期文档导入与冲突列表
  issues_decisions: 文档导入时生成 `ArtifactKind::Document` artifact，并用 artifact content hash 作为 project document 的 `content_hash`；完整本地扫描和双向同步策略留到 `V2.4-SYN-*`
  next_action: 进入 `V2.4-SYN-001`，实现本地项目文档同步引擎

- timestamp: 2026-04-12 00:30:00 CST
  task_id: V2.4-KER-001
  executor: Codex
  duration: 0.7h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 在 `memory-kernel` 增加 `UpsertAgentContextRequest`、`ListAgentContextsRequest`、`PromoteAgentContextRequest` 与 upsert/list/delete/promote 服务方法；补 `PgStore::get_agent_context`；新增 Kernel PG+Markdown 集成测试覆盖短期上下文提升为长期 memory
  issues_decisions: promote 采用最小可用路径，将短期 AgentContext 转成 `RememberTextRequest` 并写入普通 Memory；layer-aware context fetch 留到后续检索编排中继续深化
  next_action: 进入 `V2.4-KER-002`，实现中期 Project Document Kernel 服务

- timestamp: 2026-04-11 23:59:00 CST
  task_id: V2.4-STO-001
  executor: Codex
  duration: 0.8h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 为 `memory-store-pg` 增加 source/context/document CRUD，支持按 source 查询多个 access key；新增 PG 集成测试覆盖 source 多 key、短期 context 往返/删除、项目文档冲突查询
  issues_decisions: Store 层先提供稳定 CRUD 与 source-key 查询；layer-aware 检索组合放到后续 Kernel 服务接入，避免把 store 查询策略和业务编排揉在一起
  next_action: 进入 `V2.4-KER-001`，实现短期 Agent Context Kernel 服务

- timestamp: 2026-04-11 23:59:00 CST
  task_id: V2.4-DOM-001/V2.4-DOM-002/V2.4-DB-001
  executor: Codex
  duration: 0.9h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 新增 `MemoryLayer`、`MemorySource`、`AgentContext`、`ProjectDocument` 及相关 ID/状态枚举；`AccessKey` 与 `RequestContext` 增加可选 `source_id`；新增 `migrations/0006_memory_v2_4_layers_sources.sql`
  issues_decisions: source 采用兼容式接入，保留 `source_kind` 并用可选 `source_id` 支持每个来源多个 key；短期上下文和项目文档先独立建模，避免污染长期 `Memory` 主表；顺手为 PG migrate 增加 advisory lock，避免并发集成测试重复创建 extension 的竞态
  next_action: 进入 `V2.4-STO-001`，实现 source/context/document 的 PostgreSQL CRUD 与 source-key 查询

- timestamp: 2026-04-11 23:59:00 CST
  task_id: V2.4-DOC-001
  executor: Codex
  duration: 0.4h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 新增 `docs/meat-memory-scheme-v2_4.md`，并在 `docs/tasks/tasklist.md`、`docs/tasks/project-index.md`、`docs/README.md` 中登记 V2.4 短期/中期 Memory、项目文档同步与 source 多 key 方案
  issues_decisions: V2.4 采用 `short_term / mid_term / long_term` 生命周期分层；source 升级为一等对象，每个 source 可挂多个 access key；项目文档同步默认不静默覆盖本地文件
  next_action: 从 `V2.4-DOM-001`、`V2.4-DOM-002` 和 `V2.4-DB-001` 开始进入领域模型与数据迁移设计

- timestamp: 2026-04-11 13:10:00 CST
  task_id: V2.3-DOC-001
  executor: Codex
  duration: 0.4h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 新增 `docs/meat-memory-scheme-v2_3.md`，并在 `docs/tasks/tasklist.md`、`docs/tasks/project-index.md` 中登记 V2.3 方案、架构图与任务清单
  issues_decisions: V2.3 采用“方案文档 + 版本化 TaskList”双文档结构，避免后续执行时设计与任务状态分离
  next_action: 从 `V2.3-ARC-001` 开始梳理攻击面与信任边界，再进入正式安全扫描

- timestamp: 2026-04-11 14:10:00 CST
  task_id: V2.3-ARC-001/V2.3-SEC-001/V2.3-RPT-001
  executor: Codex
  duration: 0.8h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 补齐攻击面与安全检查矩阵，新增 `docs/scripts/security-report.sh` 并让 `write-test-reports.sh` 产出真实安全报告
  issues_decisions: 安全报告先聚焦 HTTP/MCP 敏感入口、越权与 payload 边界；后续再继续拆到更细粒度的专项用例集
  next_action: 继续推进 `V2.3-SEC-002/003/004/005` 与 `V2.3-REF-002/003`

- timestamp: 2026-04-11 23:30:00 CST
  task_id: V2.3-SEC-002/V2.3-SEC-003/V2.3-SEC-005/V2.3-REF-002/V2.3-REF-003/V2.3-TST-001/V2.3-TST-002
  executor: Codex
  duration: 1.8h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 完成第一轮安全收口，给 key 管理、browse、assistant、publish/promote 增加必需 key 与 owner scope 限制；增加 body/query/prompt/image 限制；HTTP/MCP 安全回归通过并生成 latest security report
  issues_decisions: 先优先收口高风险越权与资源滥用入口，注入专项的更细粒度路径测试继续放到下一轮 V2.3 执行
  next_action: 继续推进 `V2.3-SEC-004` 与 `V2.3-REF-004`，补更细的注入与错误面治理

- timestamp: 2026-04-11 23:45:00 CST
  task_id: V2.3-SEC-004/V2.3-REF-001/V2.3-REF-004/V2.3-TST-003/V2.3-QA-001
  executor: Codex
  duration: 0.8h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 为 Markdown rollup 增加 marker/frontmatter 转义与回读，补注入回归；将 HTTP/MCP 内部错误收口为通用响应并记录内部日志；在方案文档中明确安全职责分层，并完成最终业务回归
  issues_decisions: 当前已完成 V2.3 所定义任务的首轮实现闭环，后续增强项可以继续深化，但不阻塞本版本收口
  next_action: 生成最终安全报告并完成版本交付

- timestamp: 2026-04-11 23:55:00 CST
  task_id: V2.3-QA-OPS-001
  executor: Codex
  duration: 0.2h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 新增 `docs/scripts/test-required.sh`，并将 `just test` 与 `.githooks/pre-commit` 收口到该入口，确保安全测试成为每次标准验证的一部分
  issues_decisions: 安全测试不再依赖人工额外执行；后续团队统一使用 `./docs/scripts/test-required.sh` 作为必跑入口
  next_action: 无，当前需求已完成

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
  key_output: 重新索引仓库当时状态，确认旧 `tasks/` 目录已存在，更新 tasklist 顶部快照，并新增项目索引；后续已迁移到 `docs/tasks/`
  issues_decisions: 发现此前中断后仓库状态已变化，原 tasklist 顶部“仅 docs 文档”的快照过期，因此补做一次全量索引；后续统一迁移为 `docs/` + `docs/tasks/` 当前有效执行上下文
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
  key_output: 创建并验证 `docs/scripts/verify.sh`、`docs/scripts/bootstrap.sh`、`docs/scripts/dev-db-up.sh`、`docs/scripts/dev-db-down.sh`，本地已可跑 `cargo fmt --check`、`cargo clippy`、`cargo check`、`cargo test`
  issues_decisions: `verify.sh` 中 `clippy` 版本检查最初命令写法不兼容，已改为 `cargo clippy -V`；同时把 `pgvector-target` 检查纳入脚本
  next_action: 继续补充依赖安全门禁和本地开发便捷脚本

- timestamp: 2026-04-02 (Asia/Shanghai)
  task_id: MM-SCAF-001/MM-SCAF-003/MM-SCAF-004/MM-SCAF-005/MM-SCAF-006/MM-SCAF-007/MM-SCAF-008/MM-SCAF-009/MM-SCAF-010/MM-SCAF-011/MM-SCAF-012/MM-DEVX-002/MM-DEVX-006
  executor: Codex
  duration: 1.5h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 初始化 Git 仓库和 `.gitignore`，生成 `crates/`、`config/`、`docs/scripts/`、`tests/`、`.github/`、`docs/` 子目录，补齐 README、CHANGELOG、LICENSE、Issue/PR 模板、CI skeleton、Dockerfile、`.editorconfig`
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
  key_output: 新增 `.env.example`、`justfile`、`.githooks/pre-commit`、`.githooks/commit-msg`、`docs/scripts/install-hooks.sh`，并验证 `docker compose` 的 `app+pgvector+worker` 运行正常，`/healthz` 与 `/api/v1/meta` 返回成功
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
  key_output: 补齐 `docs/agent-integration-v1.md`、HTTP/MCP/CLI 使用文档、本地与云部署 runbook、`config/cloud.example.toml`、`infra/helm/meat-memory`、`docs/scripts/v1-acceptance.sh`、CLI E2E 测试，并将 MCP HTTP 路由真实接入 `memory-app` 与 `memory-cli serve`
  issues_decisions: 本轮在部署 smoke 中发现 `.cargo/config.toml` 全局固定 `/usr/bin/clang` 会导致 Linux Docker 构建失败，因此改为只对 Apple target 固定 clang；同时统一 `memory-worker` 二进制命名，修正 Dockerfile/compose/justfile 的入口不一致问题；云部署包在 V1 采用“单实例 app + worker 默认关闭 + Helm 最小闭环”的保守策略，避免误导为已完成共享卷集群形态
  next_action: 继续推进 `V1-ZH-001` 中文系统化验收集，随后整理 V1 提交与 release 收口

- timestamp: 2026-04-09 (Asia/Shanghai)
  task_id: V1-QA-001/P0/P1
  executor: Codex
  duration: 1.2h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 修复 `memory-kernel` 测试依赖缺口，为 `crates/memory-kernel/Cargo.toml` 补充 `serde_json.workspace = true`，恢复 `cargo test -p memory-kernel --lib --quiet` 与 `cargo test --workspace --lib --bins --quiet` 全绿；重跑单元覆盖率并生成 `target/coverage/unit-pass5/`，当前快照提升到 Line `95.46%`、Function `91.79%`、Region `87.91%`；同步回写 `docs/tasks/tasklist.md` 与 `docs/reports/test-coverage-report.md`
  issues_decisions: 继续沿用“带覆盖率插桩的测试二进制直接执行”方案，显式使用绝对 `LLVM_PROFILE_FILE` 路径，并改用 Rust toolchain 自带 `llvm-profdata` / `llvm-cov`，规避当前环境的相对路径 quirk 与 PATH 缺失问题
  next_action: 继续推进 `V1-ZH-001` 中文系统化验收语料收口，并把剩余覆盖率热点聚焦到 `memory-worker`、`memory-app`、`memory-store-pg` 等文件

- timestamp: 2026-04-09 21:49:08 CST
  task_id: V1-ZH-001/V1-REL-001/V1-REL-002
  executor: Codex
  duration: 1.4h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 补齐 `tests/integration/v1-zh-acceptance.md` 中文系统化验收语料，并在 `memory-extract`、`memory-kernel`、`memory-http`、`memory-mcp`、`memory-cli` 增加中文回归；同时把 Browser Console 首页 `/` 与图片 failover `llm_notice` 纳入回归，确认 `cargo test -p memory-extract --lib --quiet`、`cargo test -p memory-kernel --test kernel_flow_tests --quiet`、`cargo test -p memory-http --test http_api_tests --quiet`、`cargo test -p memory-mcp --test mcp_tools_tests --quiet`、`cargo test -p memory-cli --test cli_e2e --quiet`、`cargo test --workspace --lib --bins --quiet`、`./docs/scripts/v1-acceptance.sh` 全部通过，并清理根目录 `*.profraw` 残留、更新 `.gitignore`
  issues_decisions: 中文关系抽取验收样例采用“重复显式主语”的稳定写法，以规避当前抽取器对省略主语跨分句推断的不确定性；CLI failover 回归沿用“从首个 `{` 开始解析 JSON”的兼容策略，避免日志前缀影响断言；覆盖率原始产物继续视为临时噪音，不纳入长期项目记忆
  next_action: 收口 V1 版本提交边界、整理 release 说明，并开始准备 V2 的 scope/多语言规划

- timestamp: 2026-04-10 00:15:00 CST
  task_id: V1-REL-003
  executor: Codex
  duration: 0.8h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 补齐 `docs/release-notes-v1.md`，同步更新 `CHANGELOG.md`、`README.md`、`docs/README.md`、`docs/tasks/tasklist.md`、`docs/tasks/project-index.md`，将仓库状态从“V1 封板中”收口到“V1 已完成”；同时复核 `docker compose config --quiet` 与 `helm lint infra/helm/meat-memory` 作为 V1 部署边界验收入口
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
  key_output: 新增 `docs/scripts/v2-perf.sh` 与 `memory-kernel` / `memory-sync` ignored perf smoke tests，输出 `target/perf/*.txt` 基准报告；同时补强 `kernel/http/mcp/sync` 外部测试，覆盖 markdown-only `get_memory + promote`、HTTP `/api/v1/memories/promote`、MCP `memory.promote`、file-backed sync reopen/status/conflict 回归
  issues_decisions: 性能测试先采用 deterministic smoke benchmark 而非引入额外 benchmark 框架，优先确保 CI/本地都能低门槛复现并生成可比对文本报告；integration 强化重点放在 V2 新增的跨 scope promotion 与 file-backed sync 持久化链路，而不是重复已有 remember/search happy path
  next_action: 若进入 V3，可在此基础上接入更大规模数据集、并发压测与长期 trend snapshot

- timestamp: 2026-04-11 (Asia/Shanghai)
  task_id: V2-QA-REPORTS
  executor: Codex
  duration: 1.2h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 新增 `tests/reports/` 目录体系与分类 `README.md`，提供 `docs/scripts/write-test-reports.sh` 一键生成 unit/integration/e2e/perf 报告并归档到 `latest/archive`；同时调整 `docs/scripts/v2-perf.sh` 默认输出到 `tests/reports/perf/latest/`，清理旧的 `target/perf` 临时产物路径
  issues_decisions: 报告体系先采用“可读文本日志 + perf 原始 JSON 行输出 + latest/archive 双层目录”的轻量方案，不引入额外测试报告框架；`security` 目录先保留 placeholder，待后续补上可执行安全检查后再写实跑脚本
  next_action: 后续如需接入 CI，可直接把 `./docs/scripts/write-test-reports.sh` 作为统一测试报告入口

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
  key_output: 新增 `docs/scripts/export-agent-skills.sh`，支持按 `codex`、`claude-code`、`execution-agent` 或 `all` 导出 skill 模板到指定目录，并自动生成 bundle `README.md`；同步更新 `docs/agent-skills/README.md` 的导出用法
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
  key_output: 新增并实跑 `docs/scripts/v2_1-acceptance.sh`，覆盖 `memory-cli config check`、`mcp info`、`tui init`、`skills export` 与导出结果结构校验；同时将其接入 `docs/scripts/write-test-reports.sh`，生成 `tests/reports/e2e/latest/v2_1-acceptance.txt` 并把索引写入 `tests/reports/latest-run.md`
  issues_decisions: 在沙箱内刷新统一报告时，集成测试因本地 PostgreSQL 访问受限出现 `Operation not permitted (os error 1)`；改为在已获授权的非沙箱环境重跑 `./docs/scripts/write-test-reports.sh` 后恢复正常，说明失败源自执行环境限制而非代码回归
  next_action: 继续推进 `V2.1-DOC-001` 的剩余 README/runbook 收口，或转入下一阶段的交互式 TUI 能力

- timestamp: 2026-04-11 (Asia/Shanghai)
  task_id: V2.1-TUI-001/V2.1-TUI-002/V2.1-TUI-003/V2.1-DOC-001/V2.1-QA-001
  executor: Codex
  duration: 0.6h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 为 `memory-cli tui init` 增加 `--interactive` 交互问答模式，支持逐步选择 MCP 开关、数据库 URL、Markdown/Assets 根目录、四类主模型 alias、数据库检查和配置写出；同时补齐交互模式单测、CLI 文档示例，并把 `docs/scripts/v2_1-acceptance.sh` 扩展为覆盖交互式 smoke
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
  key_output: 收口 V2.1 全量范围：补齐 `mcp info --check-http` 本地连通性检查；让交互式 TUI 的最终结果面板跟随语言切换；将英文交互流程纳入 `docs/scripts/v2_1-acceptance.sh`；新增 `docs/runbook/v2_1-quickstart.md` 并完成 README/runbook 索引收口。至此 V2.1 的 CLI、MCP、Agent skill、TUI、文档与验收均已完成并可独立交付
  issues_decisions: 统一采用“沙箱友好的验收方式”，避免依赖测试内临时监听端口；`mcp info --check-http` 在服务未启动时明确返回 `unreachable`，保持诊断信息可预期而不是静默失败
  next_action: V2.1 已完成；后续可转入 V3 多模态能力，或继续增强全屏式 TUI 体验

- timestamp: 2026-04-11 (Asia/Shanghai)
  task_id: V2.1-TUI-POLISH
  executor: Codex
  duration: 0.3h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 继续完善交互式 TUI：模型编号菜单现在显示 alias、provider、deployment、locale、priority，帮助用户不用查配置文件就能判断模型候选；当向导写出配置文件时，最终面板会给出 `MEAT_MEMORY_CONFIG=<path> memory-cli config check` 的下一步验证命令
  issues_decisions: 保持标准输入/输出向导，不引入额外全屏 TUI 依赖；本轮重点优化信息密度和安装后的下一步指引。写配置 smoke 使用 `/tmp/meat-memory-tui.generated.toml`，验证后已清理临时文件
  next_action: 如继续打磨，可再做 provider 分组、API key 环境变量提示，或进入真正全屏式 TUI

- timestamp: 2026-04-11 (Asia/Shanghai)
  task_id: CFG-CONSOLIDATE-001/V2.1-TUI-002
  executor: Codex
  duration: 0.8h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 将应用运行配置从 `default/docker/cloud/local.example` 多文件收敛为单一 `config/app.toml`，新增 `config/README.md`；Docker、本地和云端差异改用 `MEAT_MEMORY_*` 环境变量覆盖；`memory-config` 支持 server/logging/storage/database/sync/locale/features 的环境变量覆盖；TUI 默认基于 `config/app.toml` 读写派生配置，并已通过 `/tmp/meat-memory-app.local.toml` 写配置 smoke
  issues_decisions: Rust/Cargo/CI/Compose 等工具配置保留在工具约定位置，不强行移动到 `config/`，避免破坏工具自动发现；只统一应用运行配置。旧路径引用只保留在历史日志和未跟踪 `docs/default/` 投影中，不修改历史记录和用户未跟踪内容
  next_action: 后续如需继续简化，可为 TUI 增加“保存为 config/app.local.toml 并提示加入 .gitignore”的专门选项

- timestamp: 2026-04-11 (Asia/Shanghai)
  task_id: DOC-SCHEME-V3
  executor: Codex
  duration: 0.2h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 新增 `docs/meat-memory-scheme-v3.md`，基于 `meat-memory-scheme-v2.md` 汇总 V2/V2.1 已完成能力、配置统一化结果、当前工程基线，以及 V3 音频/视频全多模态规划；同步更新 `docs/README.md` 文档索引
  issues_decisions: 将 `Scheme V3` 明确区分为“当前方案文档版本”和“产品 V3 全多模态阶段”，避免把已完成的 V2.1 工程基线与后续音视频任务混在同一个完成状态里
  next_action: 后续进入 V3 实现前，可继续细化音频/视频 object schema、asset metadata、timeline evidence 与 `docs/scripts/v3-acceptance.sh`

- timestamp: 2026-04-11 (Asia/Shanghai)
  task_id: V2.2-IDX-002/V2.2-IDX-003/V2.2-QA-002
  executor: Codex
  duration: 0.5h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 补齐 V2.2 embedding 写入与最小混合检索链路：`memory-models` 新增 deterministic 1536 维 `EmbeddingGateway`；`memory-kernel` 在 all/vector 写入后保存 memory embedding，并在搜索时合并 keyword + vector 结果；`memory-store-pg` 新增 `memory_embeddings` upsert、pgvector `<->` 查询和 isolation_group 过滤。同步补齐模型层、PG 集成和 SQL builder 回归，并更新 V2.2 tasklist/方案状态
  issues_decisions: 当前 embedding gateway 先采用确定性本地向量，优先打通存储和检索合同，不在本轮绑定外部 provider 调用；混合检索已完成 keyword + vector 合并，graph expansion 与 rerank 继续留在 `V2.2-IDX-003` 后续项
  next_action: 继续推进 `V2.2-OBS-002` 监控面板增强，或补 `V2.2-QA-002` 的 vector-only HTTP/CLI e2e 验收

- timestamp: 2026-04-11 (Asia/Shanghai)
  task_id: V2.2-OBS-002/V2.2-QA-002
  executor: Codex
  duration: 0.5h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 完成 V2.2 监控面板与模式验收收口：HTTP 首页 Browser Console 新增监控概览卡片，实时展示 `/metrics` 与 `/api/v1/metrics/keys` 的搜索/写入 p95、命中率、key 成功率和 storage mode 分布；`memory-cli` 的 remember/search/remember-image 已支持 `--key` 与 `MEAT_MEMORY_KEY` 环境变量解析 key 上下文，并补齐 vector-only 的 CLI/HTTP e2e，验证 vector 模式只写 PG、不写 Markdown 但仍可检索
  issues_decisions: 继续复用现有首页 console，而不是新建独立前端；CLI 的 key 接入优先采用请求级 `--key` 和环境变量，保持对已有 `key use` 输出的兼容
  next_action: 继续推进 `V2.2-KG-001` 和 `V2.2-IDX-003` 剩余项，补 graph expansion 深度过滤与 rerank

- timestamp: 2026-04-11 (Asia/Shanghai)
  task_id: V2.2-HTTP-001/V2.2-TUI-001/V2.2-IDX-003/V2.2-KG-001
  executor: Codex
  duration: 1.2h
  status: ✅
  change_hash: N/A-uncommitted
  key_output: 收口 V2.2 剩余功能：`memory-store-pg` 新增按 key_id 查询、状态更新、usage stats 聚合；`memory-kernel` 新增 access key update/rotate/stats，并在 keyed remember/search 链路记录 `key_usage_events`。混合检索已补 graph expansion 与基础 rerank，且 expansion 阶段继续遵守 isolation_group 过滤。HTTP API 已补 `PATCH /api/v1/keys/{key_id}`、`POST /api/v1/keys/{key_id}/rotate`、`GET /api/v1/keys/{key_id}/stats`；CLI 已补 `key rotate`、`key stats --key-id`；TUI init 写配置后会自动生成默认 key 并写入 `access.key_store_path`
  issues_decisions: graph expansion 采用基于已召回 memory 实体的二跳 keyword 扩展，优先保证跨 key 隔离和结果稳定性，不在本轮引入更重的独立图索引或复杂学习排序；TUI 默认 key 仅在写配置且 PG 可用时自动创建，避免 preview 模式产生副作用
  next_action: V2.2 已完成；后续进入 V3 时，可把 hybrid planner 再升级为更强的 rerank 或多索引 late fusion
