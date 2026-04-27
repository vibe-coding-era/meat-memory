# Migrations Index

本目录保存 PostgreSQL schema 的版本化迁移脚本。

## 当前迁移

- `0001_init_scopes.sql`：初始化 scope 相关结构
- `0002_init_content.sql`：初始化核心内容表
- `0003_scope_governance.sql`：补充 scope 治理能力
- `0004_memory_v2_metadata.sql`：加入 V2 metadata 结构
- `0005_access_keys.sql`：加入 access key 与访问控制相关结构
- `0006_memory_v2_4_layers_sources.sql`：加入 V2.4 layer / source 能力
- `0007_memory_v2_7_lifecycle.sql`：加入 V2.7 记忆生命周期审计事件
- `0008_memory_v2_8_governance.sql`：加入 V2.8 项目识别、提案、关系、蒸馏与版本治理结构
- `0009_memory_v2_91_benchmark.sql`：加入 V2.91 benchmark suite / run / case result 结构
- `0010_memory_v2_92_recall_trace.sql`：加入 V2.92 recall trace / candidate / budget pack 结构
- `0011_memory_v2_93_secret_health.sql`：加入 V2.93 secret finding / health report 结构

## 维护约定

- 新迁移按递增编号追加，不回写历史文件
- 迁移命名尽量体现版本与主题
- 涉及对象模型变化时，同时更新相关架构或 runbook 文档
