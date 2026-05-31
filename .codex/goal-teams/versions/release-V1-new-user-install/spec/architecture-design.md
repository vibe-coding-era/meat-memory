# Architecture Design: release-V1 新用户安装链路

```text
GitHub tag release-V1
  -> raw.githubusercontent.com/.../deploy/release-V1/install.sh
  -> GitHub Actions Release workflow
  -> GitHub Release assets:
       meat-memory-<os>-<arch>.tar.gz
       meat-memory-<os>-<arch>.tar.gz.sha256
  -> install.sh checksum verify
  -> ~/.local/bin/memory-cli
  -> ~/.config/meat-memory/app.toml
       markdown.root = ~/.local/share/meat-memory/markdown
       assets.root = ~/.local/share/meat-memory/assets
       enable_pg = false
  -> markdown-only first memory write smoke
```

## 关键约束

- `release-V1` ref 必须存在，否则 README 中的 raw installer URL 直接 404。
- GitHub Release assets 必须存在，否则安装脚本无法下载 tarball。
- 默认开发配置启用 PostgreSQL；安装器必须在首次安装时生成 markdown-only quickstart 配置，避免新用户被数据库阻塞。
