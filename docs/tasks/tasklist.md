# Meat Memory 版本化 TaskList

更新时间：2026-04-14

## 规划原则

- 本文件从“原子级全量拆解”切换为“版本化交付清单”。
- 从现在开始，`V1 / V2 / V2.1 / V2.2 / V2.7 / V3` 是范围管理的唯一主视图。
- 历史执行记录仍保留在 `docs/tasks/task-log.md`，其中旧的 `MM-*` 编号继续有效。
- 新任务优先使用 `V1-*`、`V2-*`、`V2.7-*`、`V3-*` 编号。
- 本文件只描述版本目标、交付范围、任务状态和建议顺序，不等同于实现日志。

## 当前快照

- 已完成底座：Rust workspace、PGSQL + Markdown 双存储、kernel `remember/search/publish`、知识图谱抽取、HTTP/CLI/MCP 接入层、基础 observability、sync oplog/merge baseline、V1 多模型能力抽象与 provider/model/route registry、V1 图片资产存储与寻址基线、V1 `remember_image` 写入链路、V1 最小图片理解链路、Agent 接入文档、Docker/Helm 部署骨架、V1 验收脚本与文档包、中文系统化验收语料、Browser Console / failover 回归入口，以及 V1 release notes / 封板说明。
- 当前复核状态：已确认 `memory-extract`、`memory-kernel`、`memory-http`、`memory-mcp`、`memory-cli` 定向回归，`cargo test --workspace --lib --bins --quiet`、`./docs/scripts/v1-acceptance.sh`、`docker compose config --quiet`、`helm lint infra/helm/meat-memory` 通过，V1 已完成。
- 最新单测覆盖率快照：Line `95.46%`、Function `91.79%`、Region `87.91%`，产物位于 `target/coverage/unit-pass5/`。
- 当前封板状态：`v2.5` 已完成安装、打包与部署收口；V2.4 的 Agent 实时上下文、项目文档同步、source 多 key、Markdown projection、监控统计、验收脚本和分发部署主链路均已完成。
- 当前进行中范围：`v2.6` 新项目进入时的项目记忆边界确认、TUI/CLI 数字化向导、skill 接入规则与文档收口。
- 当前最重要的未完成范围：V2.7 生命周期系统；V3 音频 / 视频多模态；npm 与 Homebrew 等公共发布渠道需要等待正式 release URL、npm scope 和 tap 地址后再接入真实发布。

## 版本范围总览

| 版本 | 目标范围 | 明确不做 |
|---|---|---|
| V1 | PGSQL + Markdown、云或本地独立部署、为混合部署预留接口、支持 Codex/Claude Code/TRAE/Qoder 与 OpenClaw/CoWork/QoderWork、跨大模型抽象、文本+图片、中文优先、知识图谱 | 不做多团队/个人隔离与合并，不做英文，不做音频/视频 |
| V2 | 多团队/个人隔离与合并、多语言扩展到中/英 | 不做音频/视频 |
| V2.1 | 强化 CLI / MCP 实用性、补齐多 Agent skill、提供安装后可快速配置系统的 TUI | 不做音频/视频，不做完整 GUI 桌面端 |
| V2.2 | 增强用户端使用体验：key 申请、多 key 来源管理、权限隔离、存储模式、跨 key 图谱/索引、监控面板 | 不做完整桌面端，不在 MVP 中强求完整向量召回质量优化 |
| V2.3 | 全面安全扫描、劫持/注入专项验证、统一安全边界与代码优化 | 不做技术路线重写，不做偏离当前分层的大规模重构 |
| V2.4 | 增加短期和中期 Memory 能力：Agent 实时上下文、项目文档同步、每个来源多个 key | 不重写长期 Memory 主链路，不默认自动覆盖本地文档，不提前做音频/视频 |
| V2.5 | 安装、打包与部署封板：二进制、Agent skills、cargo install、npm/Homebrew 骨架、Docker、Compose、systemd、Helm、release pipeline 与用户入口 | 不宣称 npm / Homebrew 已公开发布，不提前做音频/视频 |
| V2.6 | 新项目进入时先确认项目记忆边界：数字选择新/旧项目、互通/隔离、团队/个人，并通过 TUI/CLI/skill 建立 key 与 scope 绑定 | 未经确认不改 MCP 工具面，不新增数据库表，不做完整 GUI |
| V2.7 | 把已存在的短期、中期、长期三层记忆骨架补成可演进、可治理、可解释的生命周期系统 | 不进入音频/视频，不推倒当前三层对象，不默认做高风险自动覆盖或自动删除 |
| V3 | 补齐全多模态，完成音频/视频 | 无 |

## V1 交付清单

### V1 版本验收标准

- 支持 PostgreSQL 作为结构化主存，Markdown 作为 projection 和本地侧车。
- 支持本地独立部署和云端独立部署。
- 为混合部署保留 sync/oplog/merge 抽象接口，但不要求 V1 完成完整混合复制执行。
- 支持文本记忆和图片记忆。
- 支持中文优先的记忆抽取、检索和知识图谱。
- 支持 HTTP、CLI、MCP 三类 Agent 接入面。
- 支持跨大模型 provider 抽象层，但不要求一次性完成全部模型深度适配。

### V1 任务表

| ID | 任务 | 状态 | 说明 |
|---|---|---|---|
| `V1-STO-001` | PostgreSQL 主存链路 | done | 已完成 schema、写入、检索、发布更新和集成测试 |
| `V1-STO-002` | Markdown projection 链路 | done | 已完成 frontmatter、rollup、parser、roundtrip 测试 |
| `V1-KER-001` | `remember_text` / `search_context` / `publish_memory` | done | 已完成 kernel 主链路和 remember→search→publish 测试 |
| `V1-KG-001` | 知识图谱基线：memory/entity/relation 抽取 | done | 已完成 distill、entity、relation、graph context |
| `V1-API-001` | HTTP API：`/memories`、`/context/search`、健康检查、metrics | done | 已完成 HTTP handler 与集成测试 |
| `V1-API-002` | CLI：`remember`、`search`、`serve` | done | 已完成真实命令与 smoke |
| `V1-API-003` | MCP：`remember`、`search`、`fetch_context`、`publish` | done | 已完成 dispatcher、stdio/http transport skeleton 与集成测试 |
| `V1-API-004` | Agent 接入说明，覆盖 Codex / Claude Code / TRAE / Qoder / OpenClaw / CoWork / QoderWork | done | 已完成 Agent 接入指南，并让 `memory-app` / `memory-cli serve` 在启用配置时真实挂出 MCP HTTP 路由 |
| `V1-MOD-001` | 多模型能力抽象：reasoning / extraction / vision / embedding | done | 已完成 trait、request/response、descriptor、route/fallback registry |
| `V1-MOD-002` | Provider registry 与配置层，兼容 Gemini / Claude / ChatGPT / 千问 / 豆包 / Minimax / GLM | done | 已完成 provider catalog、默认配置、启动期 registry 校验与测试 |
| `V1-MUL-001` | 图片资产存储与寻址 | done | 已完成 sha256 寻址、本地文件存储、分类目录、metadata 与回读测试 |
| `V1-MUL-002` | 图片记忆写入链路，等价于 `remember_image` | done | 已完成 kernel/HTTP/CLI 图片写入、资产落盘与集成测试 |
| `V1-MUL-003` | 图片理解最小能力：OCR / caption / vision extraction 至少一条 | done | 已完成本地 image profile 解析、vision gateway、自动 caption 与派生文本写回 |
| `V1-ZH-001` | 中文优先抽取、检索、图谱样例与测试语料 | done | 已补齐 `tests/integration/v1-zh-acceptance.md`，并覆盖 kernel / HTTP / MCP / CLI 中文回归、Browser Console 首页与图片 failover 提示回归 |
| `V1-DEP-001` | 本地独立部署包：compose + app + worker + pgvector | done | 已完成 compose 卷持久化、`memory-worker` 真实入口、本地 runbook、`docker compose config` 校验 |
| `V1-DEP-002` | 云端独立部署包：Docker/Helm/最小发布闭环 | done | 已完成 Dockerfile 修正、`.dockerignore`、`config/app.toml`、Helm chart、`helm lint` 与镜像构建验证 |
| `V1-SYN-001` | 为混合部署预留 sync/oplog/merge 抽象接口 | done | 已有 `OplogEntry`、cursor/batch、append、merge、内存 replication engine |
| `V1-OBS-001` | V1 可观测性：结构化日志、延迟/命中率指标、health/ready/live/metrics | done | 已完成第一阶段 observability |
| `V1-QA-001` | V1 端到端验收：HTTP + CLI + MCP + 文本 + 图片 + provider mock | done | 已完成 CLI E2E、HTTP/MCP 集成测试、`docs/scripts/v1-acceptance.sh` 与 provider mock 路由命中校验 |
| `V1-DOC-001` | V1 文档包：安装、部署、Agent 接入、API/MCP/CLI 使用说明 | done | 已完成 Agent/HTTP/MCP/CLI/本地部署/云部署/验收文档收口 |
| `V1-REL-003` | V1 版本封板、release notes 与交付边界收口 | done | 已补齐 `docs/release-notes-v1.md`、更新 `CHANGELOG.md` / `README.md` / `docs/tasks/*`，并将仓库状态收口到 “V1 已完成” |

### V1 剩余建议执行顺序

- 无，`V1` 已完成。

## V2 交付清单

### V2 版本验收标准

- 支持多团队 / 个人隔离与合并。
- 支持中英双语，而不是中文优先的单语形态。
- 建立更完整的共享、发布、合并、冲突处理流程。

### V2 任务表

| ID | 任务 | 状态 | 说明 |
|---|---|---|---|
| `V2-SCP-001` | 多团队 / 个人 scope 模型正式化 | done | 已补 scope owner/inherit/sync 字段、层级校验、memory owner/published_from 元数据与持久化 |
| `V2-SCP-002` | 发布、review、共享、隔离策略扩展 | done | 已补 scope-aware publish policy、review/candidate、redaction 与 HTTP/MCP promote 入口 |
| `V2-MRG-001` | 多节点 merge 策略与冲突对象保留 | done | 已补 `ConflictRecord`、冲突保留与 merge status 统计 |
| `V2-SYN-001` | 持久化 replication engine 与真实 pull/apply | done | 已新增 `FileReplicationEngine` 与持久化 oplog state |
| `V2-SYN-002` | 团队级同步、跨节点回放、审计链路 | done | 已补 sync status、worker/config runtime 入口与持久化审计状态语义 |
| `V2-LNG-001` | 英文抽取、检索、图谱支持 | done | 已补英文/中文 memory kind 推断、language_code 检测与双语 metadata 透传 |
| `V2-LNG-002` | 中英双语测试集、回归测试、文档 | done | 已补 V2 双语验收文档，并覆盖 extract/kernel/http/mcp/cli 定向回归 |

### V2 建议执行顺序

1. `V2-SCP-001`
2. `V2-SCP-002`
3. `V2-SYN-001`
4. `V2-MRG-001`
5. `V2-LNG-001`
6. `V2-LNG-002`

## V2.1 交付清单

### V2.1 版本验收标准

- 支持以 CLI / MCP 作为一线使用入口，而不仅是底层调试入口。
- 为不同 Agent 提供可直接复用的 skill / 接入模板，降低首次接入成本。
- 在安装后可通过 TUI 快速完成系统初始化与关键配置，至少覆盖 LLM、配置文件路径、数据库与功能开关。

### V2.1 任务表

| ID | 任务 | 状态 | 说明 |
|---|---|---|---|
| `V2.1-CLI-001` | CLI 使用体验升级：面向日常使用而非仅调试 | done | 已完成 `config show` / `config check`、数据库检查、交互式 TUI 初始化、输出摘要与下一步提示，CLI 已可作为安装后第一入口 |
| `V2.1-CLI-002` | MCP 使用体验升级：面向 Agent 稳定接入 | done | 已完成 `mcp info`、MCP tool 参数化说明与 `mcp info --check-http` 本地连通性检查，Agent 接入信息可直接发现与验证 |
| `V2.1-SKL-001` | 为 Codex 编写一套可直接使用的 Meat Memory skill | done | 已完成 Codex skill 源模板、`agents/openai.yaml` metadata、`assets/icon.svg`，并支持脚本与 `memory-cli skills export --target codex` 两种导出方式 |
| `V2.1-SKL-002` | 为 Claude Code / TRAE / Qoder 等 Agent 编写配套 skill / 使用模板 | done | 已完成协作型 Agent skill 源模板、`agents/openai.yaml` metadata、`assets/icon.svg`，并支持脚本与 `memory-cli skills export --target claude-code` 导出 |
| `V2.1-SKL-003` | 为 OpenClaw / CoWork / QoderWork 等执行型 Agent 编写 skill / 使用模板 | done | 已完成执行型 Agent skill 源模板、`agents/openai.yaml` metadata、`assets/icon.svg`，并支持脚本与 `memory-cli skills export --target execution-agent` 导出 |
| `V2.1-TUI-001` | 新增安装后初始化 TUI：欢迎页、环境检测、配置向导 | done | 已完成 `memory-cli tui init` 预览面板与 `--interactive` 向导，支持语言选择、profile、步骤分段、review summary 和确认 |
| `V2.1-TUI-002` | TUI 配置项：LLM provider / model route / API Key env / 数据库 / Markdown / MCP 开关 | done | 已完成参数模式与交互模式的默认 locale、MCP、数据库、Markdown、资产目录和四类模型路由配置；交互模式模型选择已升级为编号候选菜单 |
| `V2.1-TUI-003` | TUI 运维动作：配置测试、连通性检查、保存与生成推荐配置 | done | 已完成 `config check --database`、`tui init --check-database`、配置写出、review 确认和最终结果面板本地化展示 |
| `V2.1-DOC-001` | V2.1 文档包：CLI/MCP 快速使用、Agent skill 指南、TUI 使用说明 | done | 已完成 CLI/MCP 文档、Agent skill 指南、README 总览、runbook 索引和 `docs/runbook/v2_1-quickstart.md` 收口 |
| `V2.1-QA-001` | V2.1 验收：CLI / MCP / TUI / skill smoke 与回归报告 | done | 已新增并实跑 `docs/scripts/v2_1-acceptance.sh`，覆盖 `config check`、`mcp info`、`tui init`、`skills export` 与导出结果校验；统一报告已生成 `tests/reports/e2e/latest/v2_1-acceptance.txt` 并写入 `latest-run.md` |

### V2.1 建议执行顺序

1. `V2.1-CLI-001`
2. `V2.1-CLI-002`
3. `V2.1-TUI-001`
4. `V2.1-TUI-002`
5. `V2.1-TUI-003`
6. `V2.1-SKL-001`
7. `V2.1-SKL-002`
8. `V2.1-SKL-003`
9. `V2.1-DOC-001`
10. `V2.1-QA-001`

## V2.2 交付清单

### V2.2 版本验收标准

- 用户通过 MCP / CLI / Skills / HTTP 使用 Meat Memory 前必须先申请 key。
- 第一次安装初始化时提供默认 key，TUI 支持创建更多 key。
- key 支持名称、来源、团队/个人、file/vector/all 存储模式、是否完全隔离。
- 支持多个 key 的来源管理、统计与跨 key 记忆互通。
- 支持个人 memory 使用团队 memory，团队 key 不能使用个人 memory。
- 建立跨 key 的知识图谱和索引过滤规则。
- 增强监控体系，提供读写性能和 key/source/storage 维度的监控面板。

### V2.2 MVP 任务表

| ID | 任务 | 状态 | 说明 |
|---|---|---|---|
| `V2.2-DOC-001` | V2.2 方案与 TaskList 文档 | done | 已落设计入口 `docs/meat-memory-scheme-v2_2.md` 并更新版本化任务表 |
| `V2.2-DOM-001` | AccessKey 领域模型 | done | 已新增 key/source/scope/storage/isolation 枚举、AccessKey、RequestContext 与校验 |
| `V2.2-DB-001` | AccessKey 数据迁移 | done | 已新增 access_keys、key_usage_events、memory_key_links、memory_embeddings |
| `V2.2-CFG-001` | Config 默认 key 与认证开关 | done | 已支持 access 配置段、require_key 与环境变量覆盖 |
| `V2.2-POL-001` | key 权限隔离策略 | done | 已实现团队 key 阻断个人 scope、完全隔离 key 按 isolation_group 过滤 |
| `V2.2-KER-001` | RequestContext 接入 Kernel | done | remember/search/image 写入已带 key context，旧调用保持兼容 |
| `V2.2-STO-001` | storage_mode 路由 | done | file/vector/all 已决定 PG、Markdown 写入路径；file 模式可用 Markdown 搜索 |
| `V2.2-HTTP-001` | key 管理 HTTP API | done | 已完成 create/list/update/rotate/stats 与 metrics 入口 |
| `V2.2-HTTP-002` | HTTP key 认证 middleware | done | 已支持 Bearer 与 X-Meat-Memory-Key，require_key 开启时强制认证 |
| `V2.2-CLI-001` | CLI key 子命令 | done | 已完成 create/list/use/stats |
| `V2.2-TUI-001` | TUI 默认 key 与创建 key 流程 | done | 已支持 `tui key-create`，首次 init 写配置时会创建默认 key 并落到 key 文件 |
| `V2.2-MCP-001` | MCP key 上下文 | done | MCP tool 参数和 HTTP header 已支持 key |
| `V2.2-SKL-001` | Skill key 接入说明 | done | 导出模板已增加 key 环境变量与申请流程 |
| `V2.2-OBS-001` | 按 key/source/storage 统计 | done | 已扩展 metrics key snapshot，并记录 keyed operation storage mode |
| `V2.2-QA-001` | V2.2 MVP 验收 | done | 已覆盖 domain/observability/kernel/http/cli/mcp/pg store 回归 |

### V2.2 Full 任务表

| ID | 任务 | 状态 | 说明 |
|---|---|---|---|
| `V2.2-IDX-001` | memory_embeddings schema | done | 已新增 memory_embeddings schema |
| `V2.2-IDX-002` | embedding 写入链路 | done | 写入 memory 后会生成 1536 维 embedding 并保存到 PG |
| `V2.2-IDX-003` | 混合检索 planner | done | 已完成 keyword + vector + graph expansion 合并召回与基础 rerank |
| `V2.2-KG-001` | 跨 key 知识图谱过滤 | done | 已实现 isolation_group 检索过滤，并覆盖 graph expansion 过程中的跨 key 过滤 |
| `V2.2-OBS-002` | 监控面板增强 | done | 已在 HTTP 首页 console 展示搜索/写入延迟、命中率、key 成功率与 storage mode 分布 |
| `V2.2-QA-002` | vector/file/all 模式验收 | done | 已覆盖 file markdown-only、all 模式向量写读与 isolation 过滤，以及 vector-only HTTP/CLI e2e |

### V2.2 建议执行顺序

1. `V2.2-DOC-001`
2. `V2.2-DOM-001`
3. `V2.2-DB-001`
4. `V2.2-CFG-001`
5. `V2.2-POL-001`
6. `V2.2-KER-001`
7. `V2.2-STO-001`
8. `V2.2-HTTP-001`
9. `V2.2-HTTP-002`
10. `V2.2-CLI-001`
11. `V2.2-TUI-001`
12. `V2.2-MCP-001`
13. `V2.2-SKL-001`
14. `V2.2-OBS-001`
15. `V2.2-QA-001`
16. `V2.2-IDX-001`
17. `V2.2-IDX-002`
18. `V2.2-IDX-003`
19. `V2.2-KG-001`
20. `V2.2-OBS-002`
21. `V2.2-QA-002`

## V2.3 交付清单

### V2.3 版本验收标准

- 建立系统级安全扫描基线，并形成问题清单与严重级别报告。
- 对劫持、注入、越权、枚举和资源滥用建立专项测试。
- 高价值接口统一纳入鉴权、授权和输入边界控制。
- `tests/security/` 与 `tests/reports/security/latest/` 不再是占位，而是可执行、可回归、可产出。
- 代码优化与必要的架构收口不偏离现有 Rust workspace + Axum + Kernel + Store 方向。

### V2.3 任务表

| ID | 任务 | 状态 | 说明 |
|---|---|---|---|
| `V2.3-DOC-001` | V2.3 方案与实时架构图文档 | done | 已新增 `docs/meat-memory-scheme-v2_3.md`，落当前/目标架构图、风险清单、交付物与执行阶段 |
| `V2.3-ARC-001` | 攻击面与信任边界清单 | done | 已在 `docs/meat-memory-scheme-v2_3.md` 补攻击面、关键资产、信任边界与风险表 |
| `V2.3-SEC-001` | 安全检查矩阵 | done | 已在 `docs/meat-memory-scheme-v2_3.md` 补认证、授权、输入限制、审计与报告矩阵 |
| `V2.3-SEC-002` | 全量静态安全扫描 | done | 已基于当前代码输出首版问题表、严重级别与修复优先级，并据此完成第一轮收口 |
| `V2.3-SEC-003` | 劫持专项扫描 | done | 已补敏感 HTTP/MCP 入口鉴权与越权回归，覆盖 browse/promote/key 管理等高风险路径 |
| `V2.3-SEC-004` | 注入专项扫描 | done | 已补 Markdown marker/frontmatter 注入防护与回归，并覆盖 base64/输入边界专项 |
| `V2.3-SEC-005` | 资源滥用与边界值扫描 | done | 已补 body/query/prompt/image 的长度或大小限制与回归测试 |
| `V2.3-REF-001` | 统一鉴权与授权收口方案 | done | 已明确 entry/kernel/policy/store 的安全职责分界，并完成第一轮实现 |
| `V2.3-REF-002` | 高价值接口安全重构 | done | 已将 key 管理、browse、assistant、publish/promote 等高价值路径收口到统一安全边界 |
| `V2.3-REF-003` | 统一输入验证与限制 | done | 已统一 body/query/prompt/image 的校验与大小限制 |
| `V2.3-REF-004` | 错误面与审计日志收口 | done | 已将 HTTP/MCP 内部错误收口为通用响应并保留内部日志 |
| `V2.3-TST-001` | 安全测试目录补齐 | done | 已补安全测试说明、HTTP/MCP 安全回归入口与报告脚本 |
| `V2.3-TST-002` | 劫持回归测试 | done | 已覆盖无 key、越权 browse/promote、key 管理鉴权等场景 |
| `V2.3-TST-003` | 注入回归测试 | done | 已覆盖异常 base64、超大 payload、Markdown marker/frontmatter 干扰等注入/边界场景 |
| `V2.3-RPT-001` | 安全报告脚本与 latest 报告产物 | done | 已新增 `docs/scripts/security-report.sh` 并接入 `write-test-reports.sh` |
| `V2.3-QA-001` | V2.3 业务回归 | done | 已验证 HTTP/MCP/Kernel/Markdown 相关主链路不回退 |

### V2.3 建议执行顺序

1. `V2.3-ARC-001`
2. `V2.3-SEC-001`
3. `V2.3-SEC-002`
4. `V2.3-SEC-003`
5. `V2.3-SEC-004`
6. `V2.3-SEC-005`
7. `V2.3-REF-001`
8. `V2.3-REF-002`
9. `V2.3-REF-003`
10. `V2.3-REF-004`
11. `V2.3-TST-001`
12. `V2.3-TST-002`
13. `V2.3-TST-003`
14. `V2.3-RPT-001`
15. `V2.3-QA-001`

## V2.4 交付清单

### V2.4 版本验收标准

- 支持短期 Memory，用于管理 Agent 当前会话、当前任务、实时上下文和工具结果摘要。
- 支持中期 Memory，用于管理项目文档、任务文档、设计文档、README、runbook 和 API 文档等工作集。
- 本地项目文档存在时，能够通过 hash/mtime 检测同步状态，并默认不静默覆盖本地文件。
- 将来源建模为一等对象，支持每个来源绑定多个 access key，并兼容现有 `source_kind` key 路径。
- `fetch_context` 能按短期/中期/长期上下文组合结果，同时遵守 key、scope 和 isolation 策略。
- HTTP / MCP / CLI 都具备最小可用入口，并补齐 V2.4 验收脚本与测试报告。

### V2.4 任务表

| ID | 任务 | 状态 | 说明 |
|---|---|---|---|
| `V2.4-DOC-001` | V2.4 方案文档与架构图 | done | 已新增 `docs/meat-memory-scheme-v2_4.md`，定义短期/中期/长期 Memory 分层、Source 多 key、项目文档同步边界 |
| `V2.4-DOM-001` | MemoryLayer 领域模型 | done | 已新增 `short_term / mid_term / long_term` 生命周期层，并用于短期 context 与中期文档对象 |
| `V2.4-DOM-002` | MemorySource 领域模型 | done | 已新增 source 实体，支持 access key 关联可选 `source_id`，并兼容现有 `source_kind` |
| `V2.4-DB-001` | V2.4 数据迁移 | done | 已新增 `memory_sources`、`agent_contexts`、`project_documents`，并给 `access_keys` 增加 `source_id` |
| `V2.4-KER-001` | 短期 Agent Context Kernel 服务 | done | 已支持 upsert/list/delete/promote 当前 Agent 实时上下文，并覆盖 Kernel PG+Markdown 集成测试 |
| `V2.4-KER-002` | 中期 Project Document Kernel 服务 | done | 已支持 source 管理、文档导入、hash 追踪、artifact 关联、文档列表/关键词过滤与冲突查询 |
| `V2.4-SYN-001` | 本地项目文档同步引擎 | done | 已实现本地文档扫描、hash 对比、clean/changed/missing 状态计划，并支持通过 Kernel 应用同步计划导入中期文档 |
| `V2.4-SYN-002` | 文档同步冲突策略 | done | 已定义 `clean / changed / deleted / conflicted` 判定与冲突报告，Kernel 同步计划会返回 missing/conflicts 且不静默覆盖用户改动 |
| `V2.4-STO-001` | PostgreSQL Store 扩展 | done | 已实现 source/context/document CRUD、source-key 查询，并覆盖 PG 集成测试；layer-aware 查询将在 Kernel 服务中继续接入 |
| `V2.4-STO-002` | Markdown Store 扩展 | done | 已新增 project document markdown projection、frontmatter 元数据读写、本地路径映射与 Kernel 导入链路落盘测试 |
| `V2.4-API-001` | HTTP API：source 多 key 管理 | done | 已新增 source 创建/列表/详情接口、source 维度 key 创建/列表接口，并在 key 响应中返回 `source_id` 以支持每个来源多个 key |
| `V2.4-API-002` | HTTP API：Agent Context | done | 已新增短期上下文 upsert/list/delete/promote 接口，支持按 session/task 查询并可 promote 为长期 Memory |
| `V2.4-API-003` | HTTP API：Project Documents Sync | done | 已新增 source 下项目文档导入、列表、冲突列表与本地同步扫描/执行接口，支持 `dry_run` 预览与 local_root 覆盖 |
| `V2.4-MCP-001` | MCP 工具扩展：实时上下文 | done | 已新增 `memory.context.upsert`、`memory.context.list`、`memory.context.promote`、`memory.context.delete` 并覆盖 PG+Markdown 集成测试 |
| `V2.4-MCP-002` | MCP 工具扩展：项目文档 | done | 已新增 `memory.docs.sync`、`memory.docs.search`、`memory.docs.conflicts`，支持本地文档扫描导入、按 source 查询和冲突列表 |
| `V2.4-CLI-001` | CLI source/context/doc 管理命令 | done | 已新增 `source`、`context`、`docs` 子命令，覆盖 source 创建/列表/key 管理、Agent Context upsert/list/promote/delete、Project Document import/list/conflicts |
| `V2.4-CLI-002` | 本地文档同步 CLI | done | 已支持 `docs status` 与 `docs sync`，复用本地文档扫描计划并输出 planned/imported/missing/conflicts |
| `V2.4-SKL-001` | Agent Skill 文案更新 | done | 已更新 Codex/Claude/执行型 Agent skill，说明短期上下文、项目文档同步、source 多 key 用法与冲突处理原则 |
| `V2.4-OBS-001` | 监控与统计 | done | 已扩展 metrics `v2_4` 快照，并在 Kernel/HTTP 监控面板统计 source/context/docs 操作、失败、导入、missing 与冲突数量 |
| `V2.4-QA-001` | 单元与集成测试 | done | 已完成 domain、sync、store、kernel、HTTP、MCP、CLI 核心回归，覆盖 source/context/docs 与监控统计路径 |
| `V2.4-QA-002` | V2.4 验收脚本与报告 | done | 已新增并实跑 `docs/scripts/v2_4-acceptance.sh`，接入 `write-test-reports.sh`，报告产物已写入 `tests/reports/e2e/latest/v2_4-acceptance.txt` 与 `latest-run.md` |

### V2.4 建议执行顺序

1. `V2.4-DOC-001`
2. `V2.4-DOM-001`
3. `V2.4-DOM-002`
4. `V2.4-DB-001`
5. `V2.4-STO-001`
6. `V2.4-KER-001`
7. `V2.4-KER-002`
8. `V2.4-SYN-001`
9. `V2.4-SYN-002`
10. `V2.4-API-001`
11. `V2.4-API-002`
12. `V2.4-API-003`
13. `V2.4-MCP-001`
14. `V2.4-MCP-002`
15. `V2.4-CLI-001`
16. `V2.4-CLI-002`
17. `V2.4-STO-002`
18. `V2.4-SKL-001`
19. `V2.4-OBS-001`
20. `V2.4-QA-001`
21. `V2.4-QA-002`

## V2.6 项目记忆边界引导清单

### V2.6 版本验收标准

- 任意新项目接入 Meat Memory 前，Agent skill 必须先通过数字选择确认：是否新项目、是否与其他项目记忆互通、团队记忆还是个人记忆。
- 新项目必须生成或绑定新的 `scope_id` 与 access key，并明确 `owner_scope_id`、`scope_kind`、`is_fully_isolated`，避免默认写入旧项目记忆。
- 非新项目必须通过数字选择确认：列出现有记忆/项目列表，或手动输入已有 `scope_id`。
- TUI 提供同等的数字化项目记忆初始化入口，能把选择结果落到 key/scope 边界。
- CLI 提供可脚本化与可交互的项目边界初始化入口，并输出可直接用于后续 remember/search/context/docs 的 `scope_id`、`key_id` 与 raw key。
- MCP 工具面本轮默认不改；如需新增 MCP project 工具，先单独形成方案并等待确认。

### V2.6 任务表

| ID | 任务 | 状态 | 说明 |
|---|---|---|---|
| `V2.6-DOC-001` | V2.6 方案与 TaskList 收口 | done | 已明确先走 TUI/CLI/skill，MCP 工具面暂不改；任务表记录验收标准、边界与执行顺序 |
| `V2.6-CLI-001` | CLI 项目记忆初始化命令 | done | 已新增 `memory-cli project init`，支持新项目创建 scope/key、旧项目列出/手动输入、非交互 `--existing` / `--scope-kind` / `--isolated` / `--shared`，并输出 JSON/文本摘要 |
| `V2.6-TUI-001` | TUI 项目记忆边界向导 | done | 已新增 `memory-cli tui project-init`，复用数字化项目记忆向导，覆盖新/旧项目、互通/隔离、团队/个人、列表/手动输入 |
| `V2.6-SKL-001` | Agent skill 项目边界规则 | done | 已更新 Codex、Claude Code/TRAE/Qoder、执行型 Agent 三套 skill，要求写入前先确认项目边界，不默认混写 |
| `V2.6-DOC-002` | CLI/TUI 使用文档 | done | 已更新 CLI 与 MCP 文档，说明 V2.6 初始化流程和 MCP 暂不改工具面的边界 |
| `V2.6-QA-001` | V2.6 回归测试 | done | 已覆盖 CLI/TUI 新项目、旧项目、隔离/互通、团队/个人、非交互参数、JSON 输出，并通过 memory-cli 定向测试与 skill 导出 smoke |

### V2.6 建议执行顺序

1. `V2.6-DOC-001`
2. `V2.6-CLI-001`
3. `V2.6-TUI-001`
4. `V2.6-SKL-001`
5. `V2.6-DOC-002`
6. `V2.6-QA-001`

## V2.7 生命周期系统清单

### V2.7 版本验收标准

- 短期 `AgentContext`、中期 `ProjectDocument / MemorySource`、长期 `Memory` 都能映射到统一的 lifecycle-aware record 视图。
- 记录至少具备统一的 `layer`、`type`、`source`、`confidence`、`status` 语义。
- 任务结束后能生成结构化摘要，保留目标、约束、决策、结果和待办。
- 默认召回必须经过 lifecycle / privacy guard，排除 forgotten、deprecated、archived、sensitive、expired 等内容。
- 召回结果必须能解释为什么被召回。
- 系统能够标记 supersede、conflict、needs_review，并对 soft forget 生效。
- HTTP / CLI / MCP 至少有一条基础治理主链路可用，并且不破坏 V2.4-V2.6 现有入口。

### V2.7 任务表

| ID | 任务 | 状态 | 说明 |
|---|---|---|---|
| `V2.7-DOC-001` | V2.7 方案文档包 | done | 已新增 `docs/V2.7/` 目录，包含总方案、差距分析、schema、recall/governance、实施计划 |
| `V2.7-DOM-001` | 统一 `Memory Record` 视图模型 | done | 已新增统一 `MemoryRecord` 视图和 native/layer/type/source/status 字段，用于映射 `AgentContext` / `ProjectDocument` / `Memory` |
| `V2.7-DOM-002` | 统一类型、来源、置信度、状态枚举 | done | 已新增 `record_type`、`source_kind`、`confidence`、`status` 统一枚举，并兼容现有 `MemoryKind` / `MemoryState` 映射 |
| `V2.7-KER-001` | `LifecycleNormalizer` | done | 已新增 `LifecycleNormalizer`，可将三层对象标准化为统一 record |
| `V2.7-KER-002` | `RecordClassifier` | done | 已新增基于现有字段和关键词的 `RecordClassifier`，补齐 type/source 推断 |
| `V2.7-KER-003` | `TaskSummaryService` | done | 已新增结构化 `TaskSummaryService`，可从短期记录提取目标、约束、决策、变更、问题和下一步 |
| `V2.7-KER-004` | `RecallGuard` | done | 已新增 `RecallGuard`，默认过滤 archived/deprecated/needs_review/restricted/expired 记录，并接入 `search_context` |
| `V2.7-KER-005` | `RecallExplanation` | todo | 返回 why recalled、matched layer/type/scope、score |
| `V2.7-KER-006` | `BudgetPacker` | todo | 先注入 summary，再按预算补 full content |
| `V2.7-KER-007` | `EvolutionService` | todo | 标记 supersede、conflict、deprecated、needs_review |
| `V2.7-KER-008` | `ForgetService` | todo | 实现 soft forget、restore、recall exclusion |
| `V2.7-KER-009` | `AuditLogService` | todo | 记录 write/update/recall/archive/forget/delete/supersede/conflict |
| `V2.7-STO-001` | 存储层兼容改造 | todo | 在不破坏旧链路的前提下保存新增 lifecycle 元数据 |
| `V2.7-API-001` | HTTP lifecycle / governance API | todo | 提供 inspect、status change、forget、audit 查询入口 |
| `V2.7-MCP-001` | MCP lifecycle tools | todo | 暴露受控治理与召回解释能力 |
| `V2.7-CLI-001` | CLI lifecycle / governance 命令 | todo | 支持 inspect、archive、forget、restore、report |
| `V2.7-OBS-001` | Lifecycle metrics | todo | 统计 recall hit/noise、archive/forget/supersede/conflict |
| `V2.7-RPT-001` | Memory Health Report | todo | 输出热点、过期、冲突、已替代、待审核项 |
| `V2.7-QA-001` | 单元测试 | done | 已覆盖 schema/normalizer/classifier/summary/guard 的核心单测，并补跑相关 kernel 主链路验证 |
| `V2.7-QA-002` | 集成测试 | todo | 覆盖跨层 recall、summary、forget、conflict、supersede 主链路 |
| `V2.7-QA-003` | 验收脚本与报告 | todo | 形成 V2.7 acceptance 入口与 latest report |

### V2.7 建议执行顺序

1. `V2.7-DOC-001`
2. `V2.7-DOM-001`
3. `V2.7-DOM-002`
4. `V2.7-KER-001`
5. `V2.7-KER-002`
6. `V2.7-KER-003`
7. `V2.7-KER-004`
8. `V2.7-QA-001`
9. `V2.7-KER-005`
10. `V2.7-KER-006`
11. `V2.7-KER-007`
12. `V2.7-KER-008`
13. `V2.7-KER-009`
14. `V2.7-STO-001`
15. `V2.7-API-001`
16. `V2.7-MCP-001`
17. `V2.7-CLI-001`
18. `V2.7-QA-002`
19. `V2.7-OBS-001`
20. `V2.7-RPT-001`
21. `V2.7-QA-003`

## V3 交付清单

### V3 版本验收标准

- 补齐音频和视频链路。
- 多模态从“文本 + 图片”扩展为“文本 + 图片 + 音频 + 视频”。
- 检索、抽取、证据链和 Agent 工具面都覆盖全多模态。

### V3 任务表

| ID | 任务 | 状态 | 说明 |
|---|---|---|---|
| `V3-MM-001` | 音频 ingest、资产存储、转写入口 | todo | V1/V2 都不包含 |
| `V3-MM-002` | 视频 ingest、关键帧与时间轴 evidence | todo | V1/V2 都不包含 |
| `V3-MM-003` | 音频/视频 extraction pipeline | todo | 需要 speech-to-text / caption / multimodal extraction |
| `V3-MM-004` | 全多模态检索与 late fusion | todo | 统一文本、图片、音频、视频召回 |
| `V3-MM-005` | MCP / HTTP / CLI 的全多模态工具面 | todo | 在现有接入层上扩展全多模态能力 |

### V3 建议执行顺序

1. `V3-MM-001`
2. `V3-MM-002`
3. `V3-MM-003`
4. `V3-MM-004`
5. `V3-MM-005`

## V2.5 安装、打包与部署实施清单

### 目标

- 建立统一的安装入口，覆盖 `npm`、`cargo install`、预编译二进制、Homebrew、Docker。
- 建立安装后 Agent skill 的标准交付路径，覆盖 Codex、Claude Code / TRAE / Qoder、执行型 Agent。
- 建立统一的打包产物与发布流水线。
- 建立本地、单机、容器和 Kubernetes 四类部署路径。

对应方案文档：

- `docs/architecture-design/install-package-deploy-plan.md`

### 任务表

| ID | 任务 | 状态 | 说明 |
|---|---|---|---|
| `DIST-DOC-001` | 安装、打包、部署方案文档 | done | 已新增完整方案文档，明确安装渠道、打包产物、部署形态与发布顺序 |
| `DIST-PKG-001` | 预编译二进制发布方案 | done | 已补 GitHub Release 多平台 workflow、`docs/scripts/build-release-artifacts.sh` 打包脚本，以及 `dist/release/*.tar.gz + *.sha256` 产物约定 |
| `DIST-PKG-002` | npm 安装入口 | done | 已新增 `packaging/npm/` 包装层骨架，支持命令代理、平台 tarball 映射和可配置 releaseBaseUrl 下载入口 |
| `DIST-PKG-003` | cargo install 路径收口 | done | 已补 `memory-cli` crate 元数据、CLI/usage 安装后验证说明，并通过 `cargo install --path crates/memory-cli --locked` 本地安装验证 |
| `DIST-PKG-003A` | 安装后 Agent skill 交付闭环 | done | 已补 `build-agent-skills-bundle.sh`、release 附带 `agent-skills-bundle.zip`，并统一本地导出与发布交付说明 |
| `DIST-PKG-004` | Homebrew 发布方案 | done | 已新增 `packaging/homebrew/` 发布说明与 Formula 模板，明确 tap、release tarball、sha256 和后续自动化接入流程 |
| `DIST-PKG-005` | Docker 镜像规范化 | done | 已将 Dockerfile 收口为 `app-runtime` / `worker-runtime` targets，compose 与文档统一使用 `meat-memory-app:<tag>` / `meat-memory-worker:<tag>` 命名 |
| `DIST-DEP-001` | 本地 compose 部署收口 | done | 已补 `.env` 镜像变量、compose 本地 tag 约定，以及 local/cloud/docker 相关说明，统一本地试跑入口 |
| `DIST-DEP-002` | 单机 systemd 部署模板 | done | 已新增 `infra/systemd/` 模板、环境文件样例和 `docs/runbook/systemd-deploy.md`，覆盖 app / worker 单机部署 |
| `DIST-DEP-003` | Helm / Kubernetes 部署完善 | done | 已补 app/worker 分镜像、existing Secret/PVC 复用约定，并更新 Helm values 与云端部署说明 |
| `DIST-CI-001` | 发布流水线设计 | done | 已将 release workflow 收口为 tag 驱动的 release assets + agent skills + Docker 镜像发布骨架，并明确 npm / Homebrew 为后续接入点 |
| `DIST-DOC-002` | 安装文档与用户入口收口 | done | 已新增 `docs/runbook/install.md`，并在 README、docs 总览、runbook、架构设计入口和 usage guide 中统一挂载安装/打包/部署入口 |

### 建议执行顺序

1. `DIST-DOC-001`
2. `DIST-PKG-001`
3. `DIST-PKG-002`
4. `DIST-PKG-003A`
5. `DIST-PKG-003`
6. `DIST-PKG-005`
7. `DIST-DEP-001`
8. `DIST-DEP-002`
9. `DIST-DEP-003`
10. `DIST-CI-001`
11. `DIST-PKG-004`
12. `DIST-DOC-002`

## 历史任务映射

本节用于保持和 `docs/tasks/task-log.md`、`docs/tasks/project-index.md` 的连续性。

| 旧任务族 | 新版本归属 |
|---|---|
| `MM-PG-*`、`MM-MD-*` | `V1-STO-*` |
| `MM-KER-001/002/004/005/006` | `V1-KER-001` |
| `MM-EXT-*` | `V1-KG-001` |
| `MM-HTTP-*` | `V1-API-001` |
| `MM-CLI-*` | `V1-API-002` |
| `MM-MCP-*` | `V1-API-003` |
| `MM-OBS-001/002/003/006` | `V1-OBS-001` |
| `MM-SYNC-001/002/003/006` | `V1-SYN-001` |
| `MM-KER-003` | `V1-MUL-002` 的上位任务来源 |
| `MM-SYNC-004/005` | `V1-DEP-*` 与后续混合部署准备任务 |

## 当前下一步

- `v2.5` 已完成安装、打包、部署和用户入口收口。
- `v2.6` 已完成项目记忆边界引导。
- 当前建议先进入 `V2.7` 生命周期系统，再进入 `V3` 音频/视频多模态。
