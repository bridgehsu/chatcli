# ChatCLI

ChatCLI 将 Telegram 消息与本机持久化 `tmux` 终端连接起来。它只负责终端展示与输入转发；Codex 与 Cursor 仍在用户自己的 Mac 上真实运行。

```text
Telegram
   ↓
ChatCLI
   ↓
tmux
   ↓
Codex / Cursor
```

一期仅支持 macOS、Telegram、Codex CLI、Cursor CLI 和每个聊天一个活动终端。启动 CLI 时先选择工作目录；该目录不要求是 Git 仓库。

## 运行前提

- macOS；
- 已安装 Rust / Cargo、`tmux`；
- 已在当前 macOS 用户下登录 Codex CLI 与 Cursor CLI；
- 可访问 Telegram API（如网络环境需要，先配置可用代理）；
- 已准备 Telegram Bot token 和允许访问的 Telegram 用户 ID。

## 配置

配置文件仅保留在本机，不能提交到 Git：

```bash
cp config.example.yaml config.yaml
```

编辑 `config.yaml`，填写：

```yaml
telegram:
  token: "你的 Telegram Bot Token"
  allowed_user_ids:
    - 你的 Telegram 用户 ID
```

## 打包流程

在项目根目录执行：

```bash
./bin/cmd.sh build
```

该命令会按以下顺序执行：

1. `cargo fmt --check`：检查 Rust 格式；
2. `cargo test`：运行单元测试；
3. `cargo clippy -- -D warnings`：将静态检查告警视为失败；
4. `cargo build --release`：构建 release 二进制。

构建成功后的可执行文件为：

```text
target/release/chatcli
```

如只想在终端中以前台方式运行，使用：

```bash
./bin/cmd.sh run
```

前台运行适合首次验证 Bot token、网络和 Telegram 收发是否正常。按 `Ctrl+C` 即可结束当前前台进程。

## 安装流程

确认 `config.yaml` 已填写后，执行：

```bash
./bin/cmd.sh install
```

安装流程会：

1. 执行完整打包流程；
2. 创建项目内的 `logs/` 目录；
3. 把 launchd 配置安装到 `~/Library/LaunchAgents/com.chatcli.plist`；
4. 将 launchd 配置指向当前项目目录下的 `target/release/chatcli`；
5. 在当前登录会话中启动 ChatCLI 服务。

配置文件与日志仍保留在项目目录：

```text
config.yaml
logs/launchd.out.log
logs/launchd.err.log
```

安装完成后检查服务状态：

```bash
./bin/cmd.sh status
```

持续查看运行日志：

```bash
./bin/cmd.sh logs
```

## 服务管理

```bash
./bin/cmd.sh start      # 启动已安装的服务
./bin/cmd.sh stop       # 停止服务
./bin/cmd.sh restart    # 重启服务
./bin/cmd.sh uninstall  # 停止服务并删除 launchd 配置
```

默认安装**不会**配置登录自启动或崩溃自动重启。`install` 仅在执行当次启动服务；用户如需自启动，可自行修改 `~/Library/LaunchAgents/com.chatcli.plist` 并添加 `RunAtLoad`、`KeepAlive` 等 launchd 配置。

如果 `install` 曾因服务被标记为 disabled 而失败，可直接再次执行 `./bin/cmd.sh start`。启动脚本会先重新启用 `com.chatcli`，再加载并启动服务。

## Telegram 使用方式

```text
/open
→ 选择 Codex 或 Cursor
→ 选择 Home (~) 或发送工作目录
→ ChatCLI 在该目录创建 tmux 并启动 CLI
```

常用命令：

```text
/screen  查看当前终端画面
/attach  在本机 Mac Terminal 接入同一 tmux 会话
/stop    发送 Ctrl+C
/close   关闭当前 tmux 会话
```

有活动终端时，普通 Telegram 文本会直接输入当前 Codex 或 Cursor；无活动终端时，ChatCLI 会提示选择要启动的 CLI。
