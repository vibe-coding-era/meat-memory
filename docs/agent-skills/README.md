# V2.1 Agent Skill 模板

本目录保存 Meat Memory 的 Agent skill 源模板，用于后续复制到不同 Agent 平台的 skill / instruction / tool 配置目录。

当前模板：

- `codex-meat-memory/`: 面向 Codex 的项目记忆 skill
- `claude-code-meat-memory/`: 面向 Claude Code / TRAE / Qoder 的代码协作 skill
- `execution-agent-meat-memory/`: 面向 OpenClaw / CoWork / QoderWork 的执行协作 skill

建议使用方式：

1. 先运行 `memory-cli config check` 确认本地配置可用。
2. 再运行 `memory-cli mcp info` 获取 MCP 地址和工具清单。
3. 可直接导出：

```bash
./scripts/export-agent-skills.sh all ./dist/agent-skills
./scripts/export-agent-skills.sh codex /tmp/codex-skills
./scripts/export-agent-skills.sh claude-code /tmp/claude-skills
./scripts/export-agent-skills.sh execution-agent /tmp/execution-skills
```

也可以直接使用 CLI：

```bash
cargo run -p memory-cli -- skills export --target all --output-dir ./dist/agent-skills --force
```

4. 将导出的目录复制到目标 Agent 的 skill / instruction 目录。
5. 根据 Agent 平台的 MCP 配置方式，接入 `/mcp/tools` 与 `/mcp/tools/call`。

当前导出的完整 skill 包包含：

- `SKILL.md`
- `agents/openai.yaml`
- `assets/icon.svg`

当前 metadata 已补齐：

- `icon_small`
- `icon_large`
- `display_name`
- `short_description`
- `brand_color`
- `default_prompt`

导出目标说明：

- `codex`: 导出 Codex skill
- `claude-code`: 导出 Claude Code / TRAE / Qoder skill
- `execution-agent`: 导出 OpenClaw / CoWork / QoderWork 风格 skill
- `all`: 一次性导出全部模板
