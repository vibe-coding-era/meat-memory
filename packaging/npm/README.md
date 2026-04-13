# npm 安装入口

本目录保存 `Meat Memory` 的 npm 包装层。

## 当前状态

该包装层用于后续发布 `@meat-memory/cli`，但当前还不应对外宣称 npm 渠道已发布。

原因：

- 正式 GitHub 仓库地址仍未最终确认
- release 下载域名仍未最终确认
- npm 包名和 publish scope 仍需最终确认

## 包装层职责

- 提供 `meat-memory` 和 `memory-cli` 两个命令入口
- 安装期按 OS/Arch 映射 release tarball
- 下载并解压 `memory-cli` 二进制
- 运行时把参数转发给真实 `memory-cli`

## 支持平台映射

- macOS arm64：`meat-memory-darwin-arm64.tar.gz`
- macOS x64：`meat-memory-darwin-amd64.tar.gz`
- Linux arm64：`meat-memory-linux-arm64.tar.gz`
- Linux x64：`meat-memory-linux-amd64.tar.gz`

## 本地检查

```bash
cd packaging/npm
npm run check
```

如果要验证命令代理，可以先设置已有二进制路径：

```bash
MEAT_MEMORY_BINARY_PATH=/absolute/path/to/memory-cli node bin/meat-memory.js --help
```

## 发布前需要补齐

在正式发布前，需要补齐：

- `package.json` 中的最终包名
- `package.json` 中的 `config.releaseBaseUrl`
- npm registry 权限和 publish token
- release workflow 中的 npm publish job

也可以在安装时通过环境变量临时指定：

```bash
MEAT_MEMORY_NPM_RELEASE_BASE_URL=https://github.com/<owner>/<repo>/releases/download npm install -g @meat-memory/cli
```
