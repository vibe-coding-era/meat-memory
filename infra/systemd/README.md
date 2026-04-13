# systemd Templates

本目录保存单机 Linux 部署 `Meat Memory` 时可直接复用的 `systemd` 模板。

## 当前内容

- `meat-memory-app.service`：HTTP / MCP / Browser Console 主服务
- `meat-memory-worker.service`：后台 worker 服务
- `meat-memory.env.example`：两类服务共用的环境变量样例

## 推荐目录约定

- 二进制目录：`/opt/meat-memory/bin`
- 配置目录：`/etc/meat-memory`
- 数据目录：`/var/lib/meat-memory`
- 日志查看：`journalctl -u meat-memory-app -f`

## 最小使用方式

1. 把 release 包解压到 `/opt/meat-memory`
2. 复制 `meat-memory.env.example` 为 `/etc/meat-memory/meat-memory.env`
3. 复制 `config/app.toml` 到 `/etc/meat-memory/app.toml`
4. 安装 service 文件并执行：

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now meat-memory-app
sudo systemctl enable --now meat-memory-worker
```
