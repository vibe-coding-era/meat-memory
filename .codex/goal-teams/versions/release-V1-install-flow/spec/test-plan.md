# Test Plan: release-V1 安装流程

## 必跑

1. 语法检查：

```bash
bash -n deploy/release-V1/install.sh deploy/release-V1/dev-up.sh deploy/release-V1/dev-down.sh
```

2. 环境检查：

```bash
deploy/release-V1/install.sh --check --skip-tui
```

3. 本地模拟 release asset 安装：

- 构造平台匹配 tarball 和 sha256。
- 设置 `MEAT_MEMORY_RELEASE_BASE_URL` 指向本地制品目录。
- 设置临时 `MEAT_MEMORY_INSTALL_DIR` 和 `MEAT_MEMORY_CONFIG_DIR`。
- 执行 `install.sh --skip-tui`。

4. 安装结果：

```bash
memory-cli --help
memory-cli config check
memory-cli tui init --help
```

5. `dev-up.sh` 行为：

- 缺二进制时应调用 `install.sh --skip-tui`。
- 不应触发 TUI 交互。

## 可接受跳过

- 真实 GitHub Release 下载：若 `release-V1` tag/assets 不存在，记录 external blocker。
- `--install-deps` 真实系统包安装：不在用户机器上强制执行，只验证提示和参数路径。
