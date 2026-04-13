# Homebrew 发布方案

本目录保存 `Meat Memory` 的 Homebrew 发布方案和 Formula 模板。

## 当前状态

Homebrew 尚未作为已发布安装渠道对外开放，原因是正式仓库地址、tap 地址和 release 下载域名仍需最终确认。

当前可以先使用本目录做两件事：

- 在正式 release 前检查 Formula 结构
- 在确定 GitHub 仓库和 tap 后，替换模板中的占位符并发布

## 推荐 tap

建议后续使用独立 tap：

```text
homebrew-meat-memory
```

推荐安装命令：

```bash
brew tap <owner>/meat-memory
brew install meat-memory
```

或：

```bash
brew install <owner>/meat-memory/meat-memory
```

## 产物来源

Formula 应使用 GitHub Release 里的 macOS 二进制包：

- Apple Silicon：`meat-memory-darwin-arm64.tar.gz`
- Intel macOS：`meat-memory-darwin-amd64.tar.gz`

对应 checksum 来自：

- `meat-memory-darwin-arm64.tar.gz.sha256`
- `meat-memory-darwin-amd64.tar.gz.sha256`

## 发布流程

1. 打 Git tag，例如 `v0.4.0`
2. 等 release workflow 产出二进制包和 checksum
3. 下载或读取 macOS 两个 tarball 的 sha256
4. 复制 `meat-memory.rb.template` 到 tap 仓库的 `Formula/meat-memory.rb`
5. 替换：
   - `__VERSION__`
   - `__RELEASE_BASE_URL__`
   - `__DARWIN_ARM64_SHA256__`
   - `__DARWIN_AMD64_SHA256__`
6. 在 tap 仓库中执行：

```bash
brew audit --strict --new Formula/meat-memory.rb
brew install --build-from-source Formula/meat-memory.rb
brew test meat-memory
```

## 安装后验证

用户安装后建议执行：

```bash
memory-cli --help
memory-cli config check
memory-cli mcp info
memory-cli skills export --target all --output-dir ./dist/agent-skills --force
```

## 后续自动化接入点

当正式 tap 地址确定后，可以在 release workflow 中增加 Homebrew bump job：

- 从 GitHub Release artifacts 读取 macOS checksum
- 渲染 `meat-memory.rb.template`
- 向 tap 仓库提交 PR
