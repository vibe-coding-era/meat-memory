# Requirement Specification Card

## 核心目标

让 `release-V1` 部署包的安装命令更可靠：先检测环境，缺基础工具时给出修复路径，安装 release 二进制后引导用户使用 TUI 完成本地配置，并确保自动化/后台启动不会被 TUI 阻塞。

## 关键业务功能

- 环境检测：识别 OS/arch、下载工具、checksum、安装目录和配置目录。
- 依赖补齐：默认提示，显式 `--install-deps` 才尝试安装基础工具。
- 二进制安装：下载、checksum 校验、解压、安装三类入口。
- TUI 配置：安装后有交互终端时进入 `memory-cli tui init --interactive`。
- 自动化兼容：`--skip-tui` 和 `MEAT_MEMORY_RUN_TUI=0` 可跳过。
- 部署目录启动：`dev-up.sh` 使用 `--skip-tui`，不在后台场景触发交互。

## 边界

- 不负责发布真实 GitHub Release assets。
- 不伪造生产数据库、SaaS 凭据或 provider refs。
- 不修改源码。

## 验收

- 本地模拟 release asset 能完成安装。
- 真实 GitHub Release 缺失时被记录为 external blocker，而不是误判为脚本安装成功。
