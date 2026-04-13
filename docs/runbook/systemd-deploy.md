# systemd 单机部署说明

本文档说明如何在一台 Linux 主机上，以 `systemd` 方式部署 `Meat Memory`。

## 1. 适用场景

- 单台 Linux 虚机或裸机
- 团队内部自托管
- 不使用 Kubernetes，但希望服务开机自启、统一由 `systemd` 管理

## 2. 目录建议

建议约定：

- 程序目录：`/opt/meat-memory`
- 配置目录：`/etc/meat-memory`
- 数据目录：`/var/lib/meat-memory`

其中：

- `/opt/meat-memory/bin`：放 `memory-app`、`memory-worker`
- `/etc/meat-memory/app.toml`：主配置文件
- `/etc/meat-memory/meat-memory.env`：环境变量文件
- `/var/lib/meat-memory/docs`：Markdown projection
- `/var/lib/meat-memory/assets`：资产目录

## 3. 准备文件

假设你已经拿到了 release tarball：

```bash
sudo mkdir -p /opt/meat-memory /etc/meat-memory /var/lib/meat-memory/docs /var/lib/meat-memory/assets
sudo tar -xzf meat-memory-linux-amd64.tar.gz -C /opt/meat-memory --strip-components=1
sudo cp /opt/meat-memory/config/app.toml /etc/meat-memory/app.toml
sudo cp infra/systemd/meat-memory.env.example /etc/meat-memory/meat-memory.env
```

然后按实际环境修改：

- `/etc/meat-memory/app.toml`
- `/etc/meat-memory/meat-memory.env`

至少确认：

- `MEAT_MEMORY_DATABASE_URL`
- `MEAT_MEMORY_MARKDOWN_ROOT`
- `MEAT_MEMORY_ASSETS_ROOT`
- `MEAT_MEMORY_ENABLE_MCP`

## 4. 安装 systemd service

复制模板：

```bash
sudo cp infra/systemd/meat-memory-app.service /etc/systemd/system/
sudo cp infra/systemd/meat-memory-worker.service /etc/systemd/system/
```

建议先创建运行用户：

```bash
sudo useradd --system --home /opt/meat-memory --shell /usr/sbin/nologin meat-memory || true
sudo chown -R meat-memory:meat-memory /opt/meat-memory /var/lib/meat-memory /etc/meat-memory
```

然后加载并启动：

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now meat-memory-app
sudo systemctl enable --now meat-memory-worker
```

## 5. 检查

查看状态：

```bash
sudo systemctl status meat-memory-app
sudo systemctl status meat-memory-worker
```

查看日志：

```bash
sudo journalctl -u meat-memory-app -f
sudo journalctl -u meat-memory-worker -f
```

检查服务：

```bash
curl http://127.0.0.1:8080/healthz
curl http://127.0.0.1:8080/api/v1/meta
curl http://127.0.0.1:8080/mcp/tools
```

## 6. 常见调整

如果你只想先跑主服务，可以只启 `meat-memory-app`：

```bash
sudo systemctl enable --now meat-memory-app
```

如果 PostgreSQL 在外部机器，直接在环境文件里覆盖：

```bash
MEAT_MEMORY_DATABASE_URL=postgres://user:pass@db-host:5432/meat_memory
```

如果不想暴露 MCP，可以设置：

```bash
MEAT_MEMORY_ENABLE_MCP=false
```
