# Infra Index

本目录存放部署和基础设施相关资产。

## 当前内容

- `docker/`：容器初始化相关文件
- `systemd/`：单机 Linux 部署模板与环境文件样例
- `helm/`：Helm chart 骨架与默认 values

其中：

- [`docker/README.md`](docker/README.md)：本地 Docker 初始化资产说明
- [`systemd/README.md`](systemd/README.md)：单机 systemd 部署模板说明

## 适用场景

- 想看容器启动时数据库如何初始化
- 想看云端部署骨架或后续 chart 扩展点

如果你只是想先本地启动服务，优先看 [`../docs/runbook/local-deploy-v1.md`](../docs/runbook/local-deploy-v1.md)。
