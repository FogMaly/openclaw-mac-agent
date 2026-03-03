# OpenClaw Mac Agent

基于 Rust + QUIC 的轻量级远程命令执行 Agent。

## 功能特性

- ✅ QUIC 协议连接（基于 quinn）
- ✅ 自动重连机制
- ✅ 心跳保活
- ✅ 命令白名单安全检查
- ✅ 实时命令输出（流式）
- ✅ JSON 消息协议
- ✅ 编译优化（体积小、性能高）

## 编译结果

- **二进制文件大小**: 2.1 MB（已优化）
- **编译状态**: ✅ 成功
- **优化级别**: `opt-level = "z"` (最小体积)
- **LTO**: 已启用
- **Strip**: 已启用

## 项目结构

```
openclaw-mac-agent/
├── Cargo.toml          # 项目配置和依赖
├── src/
│   ├── main.rs         # 程序入口
│   ├── client.rs       # QUIC 客户端（连接、重连、心跳）
│   ├── executor.rs     # 命令执行器（白名单检查）
│   └── config.rs       # 配置管理
└── target/release/
    └── openclaw-agent  # 编译后的二进制文件
```

## 配置文件

首次运行时会自动创建配置文件：`~/.openclaw-agent/config.json`

```json
{
  "server_addr": "127.0.0.1",
  "server_port": 4433,
  "agent_id": "mac-agent-001",
  "auth_token": "default-token",
  "reconnect_interval_secs": 5,
  "heartbeat_interval_secs": 30,
  "command_whitelist": [
    "ls",
    "pwd",
    "echo",
    "date",
    "whoami"
  ]
}
```

## 使用方法

### 1. 编译

```bash
cd ~/openclaw-mac-agent
cargo build --release
```

### 2. 运行

```bash
./target/release/openclaw-agent
```

### 3. 配置

编辑 `~/.openclaw-agent/config.json` 修改：
- `server_addr`: VPS 服务器地址
- `server_port`: QUIC 服务端口
- `agent_id`: Agent 唯一标识
- `auth_token`: 认证令牌
- `command_whitelist`: 允许执行的命令列表

## 消息协议

### 认证消息
```json
{
  "type": "auth",
  "agent_id": "mac-agent-001",
  "token": "your-token"
}
```

### 心跳消息
```json
{
  "type": "heartbeat",
  "timestamp": 1234567890
}
```

### 命令请求
```json
{
  "type": "command",
  "id": "cmd-001",
  "command": "ls -la"
}
```

### 命令响应
```json
{
  "type": "response",
  "id": "cmd-001",
  "success": true,
  "output": "total 8\ndrwxr-xr-x  2 user user 4096 ...",
  "error": null
}
```

## 安全特性

1. **命令白名单**: 只允许执行配置文件中列出的命令
2. **证书验证**: 支持 TLS 证书验证（当前为测试模式，跳过验证）
3. **认证机制**: 连接时需要提供 agent_id 和 token
4. **输出限制**: 命令输出通过流式读取，避免内存溢出

## 依赖项

- `quinn`: QUIC 协议实现
- `tokio`: 异步运行时
- `rustls`: TLS 加密
- `serde/serde_json`: JSON 序列化
- `anyhow`: 错误处理
- `tracing`: 日志记录

## 下一步开发

### Phase 2: 服务端开发
- [ ] 实现 QUIC 服务端（接收 Agent 连接）
- [ ] Agent 管理（注册、心跳检测、状态监控）
- [ ] 命令分发和结果收集
- [ ] Web API 接口

### Phase 3: 功能增强
- [ ] 文件传输支持
- [ ] 多命令并发执行
- [ ] 命令执行超时控制
- [ ] 更细粒度的权限控制
- [ ] 日志上报

### Phase 4: 生产部署
- [ ] 正式的证书管理
- [ ] 配置热重载
- [ ] 性能监控和指标
- [ ] 自动更新机制
- [ ] macOS 系统服务集成（launchd）

## 故障排查

### 连接失败
- 检查服务器地址和端口是否正确
- 确认防火墙规则允许 QUIC 流量
- 查看日志输出的错误信息

### 命令执行失败
- 确认命令在白名单中
- 检查命令语法是否正确
- 查看返回的错误信息

## 许可证

MIT License
