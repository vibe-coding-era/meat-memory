# Architecture Design: release-V1 安装流程

## 流程

```text
install.sh
  -> parse args
  -> ensure_environment
  -> detect_asset_name
  -> download tarball + sha256
  -> verify checksum
  -> install binaries
  -> memory-cli --help
  -> memory-cli config check
  -> run_tui_config(auto/forced/skipped)
```

## 测试夹具

本地测试不依赖正式 GitHub Release。QA 可创建临时目录，构造与 release workflow 一致的目录结构：

```text
meat-memory-darwin-arm64/
  bin/memory-cli
  bin/memory-app
  bin/memory-worker
  config/app.toml
```

将其压缩为 `meat-memory-<platform>.tar.gz`，生成 `.sha256`，通过 `MEAT_MEMORY_RELEASE_BASE_URL=file://...` 或本地 HTTP server 测试安装器。

## 风险

- `file://` 对 `curl` 可用但 `wget` 行为可能不同；必要时用本地 HTTP server。
- `memory-cli config check` 可能因 mock binary 与真实 CLI 行为不同而只能验证调用路径；真实二进制安装仍受 GitHub Release assets 是否存在影响。
