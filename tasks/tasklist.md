# Meat Memory 版本化 TaskList

更新时间：2026-04-11

## 规划原则

- 本文件从“原子级全量拆解”切换为“版本化交付清单”。
- 从现在开始，`V1 / V2 / V3` 是范围管理的唯一主视图。
- 历史执行记录仍保留在 `tasks/task-log.md`，其中旧的 `MM-*` 编号继续有效。
- 新任务优先使用 `V1-*`、`V2-*`、`V3-*` 编号。
- 本文件只描述版本目标、交付范围、任务状态和建议顺序，不等同于实现日志。

## 当前快照

- 已完成底座：Rust workspace、PGSQL + Markdown 双存储、kernel `remember/search/publish`、知识图谱抽取、HTTP/CLI/MCP 接入层、基础 observability、sync oplog/merge baseline、V1 多模型能力抽象与 provider/model/route registry、V1 图片资产存储与寻址基线、V1 `remember_image` 写入链路、V1 最小图片理解链路、Agent 接入文档、Docker/Helm 部署骨架、V1 验收脚本与文档包、中文系统化验收语料、Browser Console / failover 回归入口，以及 V1 release notes / 封板说明。
- 当前复核状态：已确认 `memory-extract`、`memory-kernel`、`memory-http`、`memory-mcp`、`memory-cli` 定向回归，`cargo test --workspace --lib --bins --quiet`、`./scripts/v1-acceptance.sh`、`docker compose config --quiet`、`helm lint infra/helm/meat-memory` 通过，V1 已完成。
- 最新单测覆盖率快照：Line `95.46%`、Function `91.79%`、Region `87.91%`，产物位于 `target/coverage/unit-pass5/`。
- 当前最重要的未完成范围：V2.1 的 Agent 可用性增强与安装后配置体验；V2 已完成并进入稳定化。

## 版本范围总览

| 版本 | 目标范围 | 明确不做 |
|---|---|---|
| V1 | PGSQL + Markdown、云或本地独立部署、为混合部署预留接口、支持 Codex/Claude Code/TRAE/Qoder 与 OpenClaw/CoWork/QoderWork、跨大模型抽象、文本+图片、中文优先、知识图谱 | 不做多团队/个人隔离与合并，不做英文，不做音频/视频 |
| V2 | 多团队/个人隔离与合并、多语言扩展到中/英 | 不做音频/视频 |
| V2.1 | 强化 CLI / MCP 实用性、补齐多 Agent skill、提供安装后可快速配置系统的 TUI | 不做音频/视频，不做完整 GUI 桌面端 |
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
| `V1-DEP-002` | 云端独立部署包：Docker/Helm/最小发布闭环 | done | 已完成 Dockerfile 修正、`.dockerignore`、`config/cloud.example.toml`、Helm chart、`helm lint` 与镜像构建验证 |
| `V1-SYN-001` | 为混合部署预留 sync/oplog/merge 抽象接口 | done | 已有 `OplogEntry`、cursor/batch、append、merge、内存 replication engine |
| `V1-OBS-001` | V1 可观测性：结构化日志、延迟/命中率指标、health/ready/live/metrics | done | 已完成第一阶段 observability |
| `V1-QA-001` | V1 端到端验收：HTTP + CLI + MCP + 文本 + 图片 + provider mock | done | 已完成 CLI E2E、HTTP/MCP 集成测试、`scripts/v1-acceptance.sh` 与 provider mock 路由命中校验 |
| `V1-DOC-001` | V1 文档包：安装、部署、Agent 接入、API/MCP/CLI 使用说明 | done | 已完成 Agent/HTTP/MCP/CLI/本地部署/云部署/验收文档收口 |
| `V1-REL-003` | V1 版本封板、release notes 与交付边界收口 | done | 已补齐 `docs/release-notes-v1.md`、更新 `CHANGELOG.md` / `README.md` / `tasks/*`，并将仓库状态收口到 “V1 已完成” |

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

## V3 交付清单

## V2.1 交付清单

### V2.1 版本验收标准

- 支持以 CLI / MCP 作为一线使用入口，而不仅是底层调试入口。
- 为不同 Agent 提供可直接复用的 skill / 接入模板，降低首次接入成本。
- 在安装后可通过 TUI 快速完成系统初始化与关键配置，至少覆盖 LLM、配置文件路径、数据库与功能开关。

### V2.1 任务表

| ID | 任务 | 状态 | 说明 |
|---|---|---|---|
| `V2.1-CLI-001` | CLI 使用体验升级：面向日常使用而非仅调试 | doing | 已启动，先补 `config show` / `config check` 配置发现与模型路由检查入口；后续继续补交互提示与错误提示 |
| `V2.1-CLI-002` | MCP 使用体验升级：面向 Agent 稳定接入 | doing | 已启动，先补 `mcp info` 与 MCP tool 参数化说明；后续继续补连通性检查与错误返回优化 |
| `V2.1-SKL-001` | 为 Codex 编写一套可直接使用的 Meat Memory skill | doing | 已新增 Codex skill 源模板、`agents/openai.yaml` metadata、`assets/icon.svg`，并支持脚本与 `memory-cli skills export --target codex` 两种导出方式 |
| `V2.1-SKL-002` | 为 Claude Code / TRAE / Qoder 等 Agent 编写配套 skill / 使用模板 | doing | 已新增协作型 Agent skill 源模板、`agents/openai.yaml` metadata、`assets/icon.svg`，并支持脚本与 `memory-cli skills export --target claude-code` 导出 |
| `V2.1-SKL-003` | 为 OpenClaw / CoWork / QoderWork 等执行型 Agent 编写 skill / 使用模板 | doing | 已新增执行型 Agent skill 源模板、`agents/openai.yaml` metadata、`assets/icon.svg`，并支持脚本与 `memory-cli skills export --target execution-agent` 导出 |
| `V2.1-TUI-001` | 新增安装后初始化 TUI：欢迎页、环境检测、配置向导 | doing | 已新增 `memory-cli tui init` 初始化面板预览，覆盖 LLM 路由、存储、MCP 状态和下一步指引；后续升级为可写配置向导 |
| `V2.1-TUI-002` | TUI 配置项：LLM provider / model route / API Key env / 数据库 / Markdown / MCP 开关 | doing | 已支持通过 `tui init` 覆盖 MCP、数据库、Markdown、资产目录和四类模型路由，并可 `--write-config` 生成配置文件 |
| `V2.1-TUI-003` | TUI 运维动作：配置测试、连通性检查、保存与生成推荐配置 | doing | 已支持生成配置后复用 `config check` 校验模型路由，新增 `config check --database` 与 `tui init --check-database` 实时数据库连通性检查，并输出下一步提示 |
| `V2.1-DOC-001` | V2.1 文档包：CLI/MCP 快速使用、Agent skill 指南、TUI 使用说明 | doing | 已更新 CLI 文档与 Agent skill 指南，补齐 skill bundle 结构、UI metadata 与导出说明；后续继续收口 README、runbook 与配置说明 |
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

本节用于保持和 `tasks/task-log.md`、`tasks/project-index.md` 的连续性。

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

- 第一优先级：启动 V2.1，先收口 CLI / MCP 使用体验，再推进 Agent skill 与安装后 TUI 配置向导
