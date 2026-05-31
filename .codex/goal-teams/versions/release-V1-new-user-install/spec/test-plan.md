# Test Plan: release-V1 新用户安装与写入

## 测试环境

- OS: 当前机器 `uname -s`
- Arch: 当前机器 `uname -m`
- HOME: 临时目录
- Install dir: 临时目录下 `.local/bin`
- Config dir: 临时目录下 `.config/meat-memory`
- 安装来源: GitHub raw `release-V1` installer
- Release asset: GitHub Release `release-V1`

## 测试步骤

1. `curl -fsSLO https://raw.githubusercontent.com/vibe-coding-era/meat-memory/release-V1/deploy/release-V1/install.sh`
2. `chmod +x install.sh`
3. `MEAT_MEMORY_INSTALL_DIR=... MEAT_MEMORY_CONFIG_DIR=... ./install.sh --check`
4. `MEAT_MEMORY_INSTALL_DIR=... MEAT_MEMORY_CONFIG_DIR=... MEAT_MEMORY_RUN_TUI=0 ./install.sh --skip-tui --verify-write`
5. `MEAT_MEMORY_CONFIG=.../app.toml memory-cli remember ... --json`
6. `MEAT_MEMORY_CONFIG=.../app.toml memory-cli search ... --json`

## 通过标准

- 步骤 1-6 全部退出码为 0。
- installer `--verify-write` 输出 `memory write verification passed`。
- remember 输出包含 `memory_id`、`wrote_markdown=true`。
- search 输出包含 `memory_count >= 1`。
