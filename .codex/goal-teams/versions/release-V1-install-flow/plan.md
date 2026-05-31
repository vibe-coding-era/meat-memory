# release-V1-install-flow Plan

## 用户目标

先确认部署分支已提交到 GitHub，再使用 Goal Teams 测试整个安装流程，并由独立 subagent 检查安装结果是否可用。如果不可用，修复 bug 后复测。

## 已确认事实

- 当前分支：`codex/release-v1-deploy-installer`
- 本地 HEAD：`877cfdd`
- 远端 `vibe-coding-era/meat-memory.git` 同分支：`877cfdd1c92b31a409ebaf6cf01611ff0d879f83`
- 项目根存在 `AGENTS.md`。
- 项目根未发现 `CLAUDE.md`。
- GitHub Release API 查询 `release-V1` 返回 404，真实远程 release asset 安装当前预计 blocked。

## 假设

- 本轮可在本地使用模拟 release asset 验证安装器核心逻辑。
- 不创建正式 GitHub Release，不发布真实 release assets，除非用户另行确认。
- 只允许修改 `deploy/release-V1/` 和本版本 Goal Teams 运行文档。
- 不提交源码或无关脏改动。

## Done Criteria

1. `install.sh --check --skip-tui` 可完成环境检测。
2. 使用本地模拟 release asset 时，`install.sh --skip-tui` 能完成下载、checksum 校验、解压、安装三个入口。
3. 安装后的 `memory-cli config check` 可执行并给出可解释结果。
4. `memory-cli tui init --interactive` 入口可达；自动化场景能使用 `--skip-tui` 避免阻塞。
5. `dev-up.sh` 在缺少二进制时调用安装器不触发 TUI。
6. 独立 QA 和 Reviewer 均确认安装结果，或记录明确 bug / external blocker。
7. 若发现 bug，修复后复测并提交推送。

## Stop Conditions

- 需要正式 GitHub Release asset 才能继续真实远程安装。
- 需要 sudo 或系统包安装时，不在用户机器上强制执行。
- 需要修改源码或大范围发布 workflow 时，重新请求用户确认。
