# Agent Skill 模板

本目录保存 Meat Memory 的 Agent Skill 源模板，作用是把仓库里的记忆能力打包给不同 Agent 平台复用。

## 当前模板

- `codex-meat-memory/`：面向 Codex
- `claude-code-meat-memory/`：面向 Claude Code / TRAE / Qoder
- `execution-agent-meat-memory/`：面向 OpenClaw / CoWork / QoderWork

## 推荐使用顺序

1. 先检查本地配置。

```bash
cargo run -p memory-cli -- config check
```

2. 再确认 MCP 地址和工具面。

```bash
cargo run -p memory-cli -- mcp info
```

3. 导出 skill 包。

```bash
cargo run -p memory-cli -- skills export --target all --output-dir ./dist/agent-skills --force
```

也可以使用脚本：

```bash
./docs/scripts/export-agent-skills.sh all ./dist/agent-skills
./docs/scripts/build-agent-skills-bundle.sh all ./dist/release
```

4. 将导出结果复制到目标 Agent 的 skill 或 instruction 目录。
5. 按目标平台配置 MCP 的 `/mcp/tools` 与 `/mcp/tools/call`。

如果你是通过 release 包交付给用户，推荐同时附带：

- `agent-skills-bundle.zip`
- 导出的目录版 `./dist/agent-skills/`

## V2.4 相关能力

- 短期上下文：`memory-cli context ...`
- 项目文档同步：`memory-cli docs ...`
- 来源多 key：`memory-cli source ...`

最小示例：

```bash
memory-cli source create --name "Project docs" --source-kind local_docs --sync-mode index_only --local-root ./docs --json
memory-cli docs sync --source-id src_... --dry-run --json
memory-cli context upsert --session-id current --title "Current task" --body "Agent is preparing context." --json
```

如果同步结果包含 `missing` 或 `conflicts`，应该先提示用户确认，不要静默覆盖。

## 导出产物

完整 skill 包通常包含：

- `SKILL.md`
- `agents/openai.yaml`
- `assets/icon.svg`

导出目标：

- `codex`
- `claude-code`
- `execution-agent`
- `all`

标准交付物：

- 本地导出目录：`./dist/agent-skills/`
- release 附带压缩包：`./dist/release/agent-skills-bundle.zip`
