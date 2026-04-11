# Meat Memory 版本化 TaskList

更新时间：2026-04-11

## 规划原则

- 本文件从“原子级全量拆解”切换为“版本化交付清单”。
- 从现在开始，`V1 / V2 / V2.1 / V2.2 / V3` 是范围管理的唯一主视图。
- 历史执行记录仍保留在 `docs/tasks/task-log.md`，其中旧的 `MM-*` 编号继续有效。
- 新任务优先使用 `V1-*`、`V2-*`、`V3-*` 编号。
- 本文件只描述版本目标、交付范围、任务状态和建议顺序，不等同于实现日志。

## 当前快照

- 已完成底座：Rust workspace、PGSQL + Markdown 双存储、kernel `remember/search/publish`、知识图谱抽取、HTTP/CLI/MCP 接入层、基础 observability、sync oplog/merge baseline、V1 多模型能力抽象与 provider/model/route registry、V1 图片资产存储与寻址基线、V1 `remember_image` 写入链路、V1 最小图片理解链路、Agent 接入文档、Docker/Helm 部署骨架、V1 验收脚本与文档包、中文系统化验收语料、Browser Console / failover 回归入口，以及 V1 release notes / 封板说明。
- 当前复核状态：已确认 `memory-extract`、`memory-kernel`、`memory-http`、`memory-mcp`、`memory-cli` 定向回归，`cargo test --workspace --lib --bins --quiet`、`./scripts/v1-acceptance.sh`、`docker compose config --quiet`、`helm lint infra/helm/meat-memory` 通过，V1 已完成。
- 最新单测覆盖率快照：Line `95.46%`、Function `91.79%`、Region `87.91%`，产物位于 `target/coverage/unit-pass5/`。
- 当前最重要的未完成范围：V2.3 的安全与代码优化；V2.2 已完成，后续进入安全基线、专项扫描与架构收口阶段。

## 版本范围总览

| 版本 | 目标范围 | 明确不做 |
|---|---|---|
| V1 | PGSQL + Markdown、云或本地独立部署、为混合部署预留接口、支持 Codex/Claude Code/TRAE/Qoder 与 OpenClaw/CoWork/QoderWork、跨大模型抽象、文本+图片、中文优先、知识图谱 | 不做多团队/个人隔离与合并，不做英文，不做音频/视频 |
| V2 | 多团队/个人隔离与合并、多语言扩展到中/英 | 不做音频/视频 |
| V2.1 | 强化 CLI / MCP 实用性、补齐多 Agent skill、提供安装后可快速配置系统的 TUI | 不做音频/视频，不做完整 GUI 桌面端 |
| V2.2 | 增强用户端使用体验：key 申请、多 key 来源管理、权限隔离、存储模式、跨 key 图谱/索引、监控面板 | 不做完整桌面端，不在 MVP 中强求完整向量召回质量优化 |
| V2.3 | 全面安全扫描、劫持/注入专项验证、统一安全边界与代码优化 | 不做技术路线重写，不做偏离当前分层的大规模重构 |
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
| `V1-QA-001` | V1 端到端验收：HTTP + CLI + MCP + 文本 + 图片 + provider mock | done | 已完成 CLI E2E、HTTP/MCP 集成测试、`scripts/v1-acceptance.sh` 与 provider mock 路由命中校验 |
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
| `V2.1-QA-001` | V2.1 验收：CLI / MCP / TUI / skill smoke 与回归报告 | done | 已新增并实跑 `scripts/v2_1-acceptance.sh`，覆盖 `config check`、`mcp info`、`tui init`、`skills export` 与导出结果校验；统一报告已生成 `tests/reports/e2e/latest/v2_1-acceptance.txt` 并写入 `latest-run.md` |

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
| `V2.3-RPT-001` | 安全报告脚本与 latest 报告产物 | done | 已新增 `scripts/security-report.sh` 并接入 `write-test-reports.sh` |
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

- V2.3 已完成方案与任务登记，下一优先级进入攻击面梳理、安全检查矩阵与正式扫描
