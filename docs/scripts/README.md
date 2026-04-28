# 脚本说明

本目录统一存放项目的开发、验收、测试报告和辅助脚本，路径统一为 `docs/scripts/`。

## 怎么使用

所有脚本都默认从仓库根目录执行，推荐直接使用相对路径：

```bash
./docs/scripts/<script-name>.sh
```

例如：

```bash
./docs/scripts/verify.sh
./docs/scripts/dev-up.sh
./docs/scripts/test-required.sh
```

如果你平时使用 `just`，也可以继续用仓库根目录的 `justfile`，它已经指向这里的新路径。

## 常见场景

### 1. 第一次初始化开发环境

先检查本机依赖，再初始化工作区：

```bash
./docs/scripts/verify.sh
./docs/scripts/bootstrap.sh
```

适用场景：

- 新机器第一次拉项目
- 切换环境后想确认 Rust、Docker、PostgreSQL 工具链是否正常

### 2. 只启动开发数据库

```bash
./docs/scripts/dev-db-up.sh
```

停止数据库：

```bash
./docs/scripts/dev-db-down.sh
```

适用场景：

- 想本地 `cargo run -p memory-app`
- 只需要 PostgreSQL，不想拉起完整容器栈

### 3. 启动完整本地环境

```bash
./docs/scripts/dev-up.sh
```

停止：

```bash
./docs/scripts/dev-down.sh
```

适用场景：

- 本地联调 `pgvector + app + worker`
- 快速验证 HTTP / MCP 接口

### 4. 执行必跑测试

```bash
./docs/scripts/test-required.sh
```

这个脚本会执行：

- 工作区测试
- 安全专项回归入口

适用场景：

- 提交前自检
- 需要快速确认仓库是否处于可提交状态

### 5. 跑验收脚本

```bash
./docs/scripts/v1-acceptance.sh
./docs/scripts/v2_1-acceptance.sh
./docs/scripts/v2_4-acceptance.sh
./docs/scripts/v2_7-acceptance.sh
./docs/scripts/v2_8-acceptance.sh
./docs/scripts/v2_91-acceptance.sh
./docs/scripts/v2_92-acceptance.sh
./docs/scripts/v2_93-acceptance.sh
./docs/scripts/v2_94-acceptance.sh
./docs/scripts/v2_95-acceptance.sh
./docs/scripts/v2_7-production-gate.sh
./docs/scripts/v2_8-production-gate.sh
```

分别适用于：

- `v1-acceptance.sh`：V1 主链路验收
- `v2_1-acceptance.sh`：CLI / MCP / TUI / skill 验收
- `v2_4-acceptance.sh`：source / docs / context / metrics 验收
- `v2_7-acceptance.sh`：lifecycle / governance / explainability / health report 验收
- `v2_8-acceptance.sh`：proposal / timeline / rollback / profile / distillation 验收；`V2_8_ACCEPTANCE_STRICT=1` 时要求 PG-backed 主链路
- `v2_91-acceptance.sh`：V2.91 benchmark baseline、报告骨架、V2.7 / V2.8 兼容 smoke 与新增代码 100% 覆盖率说明
- `v2_92-acceptance.sh`：V2.92 recall trace、explanation、budget packer、trace CLI 和新增代码 100% 覆盖率说明
- `v2_93-acceptance.sh`：V2.93 secret / PII guard、health report、CLI health 投影和新增代码 100% 覆盖率说明
- `v2_94-acceptance.sh`：V2.94 evidence span、Memory Passport export / verify / import / provenance 和新增代码 100% 覆盖率说明
- `v2_95-acceptance.sh`：V2.95 竞品映射、adapter skeleton、CLI / HTTP / MCP surface parity 和新增代码 100% 覆盖率说明
- `v2_7-production-gate.sh`：V2.7 商用生产门禁，覆盖格式、接口、P1 回归、存储、接入面、安全和严格 PG 验收
- `v2_8-production-gate.sh`：V2.8 发布门禁，覆盖格式、接口、V2.8 核心套件、接入面、安全和 strict acceptance

### 6. 生成测试报告

统一刷新测试报告：

```bash
./docs/scripts/write-test-reports.sh
```

只生成安全专项报告：

```bash
./docs/scripts/security-report.sh
```

运行性能烟测：

```bash
./docs/scripts/v2-perf.sh
```

### 7. 导出 Agent Skill

导出全部：

```bash
./docs/scripts/export-agent-skills.sh all ./dist/agent-skills
```

只导出 Codex：

```bash
./docs/scripts/export-agent-skills.sh codex /tmp/codex-skills
```

### 8. 安装 Git Hooks

```bash
./docs/scripts/install-hooks.sh
```

执行后会把仓库的 `core.hooksPath` 指向 `.githooks`。

## 脚本清单

### 环境与启动

- `bootstrap.sh`：初始化本地 Rust 开发环境
- `verify.sh`：检查工具链、Docker、数据库等基础条件
- `dev-db-up.sh`：仅启动开发数据库
- `dev-db-down.sh`：仅停止开发数据库
- `dev-up.sh`：启动完整本地栈
- `dev-down.sh`：停止完整本地栈

### 测试与验收

- `test-required.sh`：仓库必跑测试入口
- `v1-acceptance.sh`：V1 验收
- `v2_1-acceptance.sh`：V2.1 配置 / MCP / TUI / skill 验收
- `v2_4-acceptance.sh`：V2.4 context / docs / source 验收
- `v2_7-acceptance.sh`：V2.7 lifecycle / governance / explainability / health report 验收
- `v2_8-acceptance.sh`：V2.8 proposal / timeline / rollback / profile / distillation 验收
- `v2_91-acceptance.sh`：V2.91 benchmark baseline 验收
- `v2_92-acceptance.sh`：V2.92 recall trace / budget 验收
- `v2_93-acceptance.sh`：V2.93 secret / health 验收
- `v2_94-acceptance.sh`：V2.94 evidence / passport 验收
- `v2_7-production-gate.sh`：V2.7 生产门禁
- `v2_8-production-gate.sh`：V2.8 发布门禁
- `v2-perf.sh`：性能烟测
- `security-report.sh`：安全专项报告
- `write-test-reports.sh`：统一刷新测试报告

### 开发辅助

- `export-agent-skills.sh`：导出 Agent Skill 包
- `build-agent-skills-bundle.sh`：把导出的 Agent Skill 目录打成标准 zip bundle
- `install-hooks.sh`：安装 Git hooks
- `build-release-artifacts.sh`：按目标平台打包 release tarball 和 checksum

### 打包与发布

如果你已经在本机完成某个 target 的 release build，可以用下面的脚本生成预编译二进制压缩包：

```bash
cargo build --release --locked --target x86_64-unknown-linux-gnu \
  -p memory-cli -p memory-app -p memory-worker

./docs/scripts/build-release-artifacts.sh \
  --target x86_64-unknown-linux-gnu \
  --asset-name meat-memory-linux-amd64
```

生成结果位于：

```bash
dist/release/
```

其中会包含：

- `*.tar.gz`：预编译二进制发布包
- `*.sha256`：对应 checksum 文件

## 例子

### 例子 1：本地快速试跑

```bash
cp .env.example .env
./docs/scripts/dev-up.sh
curl http://127.0.0.1:8080/healthz
curl http://127.0.0.1:8080/mcp/tools
./docs/scripts/dev-down.sh
```

### 例子 2：开发前检查

```bash
./docs/scripts/verify.sh
./docs/scripts/bootstrap.sh
./docs/scripts/test-required.sh
```

### 例子 3：刷新全部测试报告

```bash
./docs/scripts/write-test-reports.sh
ls tests/reports/e2e/latest
ls tests/reports/perf/latest
```

### 例子 4：导出给 Agent 平台使用的 skill

```bash
./docs/scripts/export-agent-skills.sh all ./dist/agent-skills
find ./dist/agent-skills -maxdepth 2 -type f | sort
```

### 例子 5：生成 release 附带的 skill bundle

```bash
./docs/scripts/build-agent-skills-bundle.sh all ./dist/release
ls ./dist/release/agent-skills-bundle.zip
```

建议先从 [`../runbook/usage-guide.md`](../runbook/usage-guide.md) 了解整体使用方式，再回到本页查具体脚本命令。
