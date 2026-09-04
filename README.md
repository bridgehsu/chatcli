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

消息会先经过 `Agent`：确定性命令与会话状态（有活动终端、等待工作目录）优先按规则处理；无活动终端时的自然语言，可在启用 `router` 后由 OpenAI 兼容模型做意图分类（如启动 CLI 引导 vs 文件定位）。模型失败时回退到关键词启发式。`Agent` 不直接执行 shell 命令，也不会编造目录或文件位置；所有终端和文件系统操作仍须由对应本地模块验证后执行。

## Agent 与 Tool 分层

`Agent` 只产生受限决策；真实本机能力集中在 `src/tools/`：

```text
Agent
  ├─ WorkspaceSearchTool：查找目录、校验路径、定位文件
  └─ TerminalTool：启动 CLI、输入、抓屏、按键注入、本机接管
```

例如用户在选择 Codex 后发送“打开 chatcli 这个目录”，`WorkspaceSearchTool` 会在本机 Home 范围内搜索同名目录：唯一结果直接用于创建 tmux；多个结果返回编号候选，用户回复序号后再启动；没有结果则提示用户重新输入路径或目录名。目录与文件结果均来自真实文件系统，不由模型编造。

一期仅支持 macOS、Telegram、Codex CLI 和 Cursor CLI。每个聊天可保留多个 tmux 会话，但任意时刻只有一个“当前会话”；启动 CLI 时先选择工作目录，该目录不要求是 Git 仓库。

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

router:
  enabled: true
  base_url: "https://api.openai.com/v1"
  api_key: ""   # 或设置环境变量 ROUTER_API_KEY
  model: "gpt-4o-mini"
  timeout_secs: 15
```

如果本机不能直连 Telegram，可在 `telegram` 下配置 HTTP 代理：

```yaml
telegram:
  proxy_url: "http://127.0.0.1:7890"
```

`router` 默认关闭。启用后需提供 API Key（`router.api_key` 或 `ROUTER_API_KEY`）；`base_url` 可为任意 OpenAI Chat Completions 兼容端点。

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
[Codex] [Cursor]
→ 进入对应 CLI 的会话中心
→ 选择“新建会话”并发送工作目录
→ ChatCLI 在该目录创建独立 tmux 并启动 CLI
→ 新会话自动成为当前会话
```

Codex 与 Cursor 分别提供“新建会话”和“所有会话”入口；列表只显示同类 CLI 会话。选择列表中的运行中会话会切换当前会话，旧会话继续在后台运行。普通 Telegram 文本始终输入当前会话。

常用操作：

```text
/screen  查看当前终端画面
/attach  在本机 Mac Terminal 接入同一 tmux 会话
/stop    发送 Ctrl+C
/close   结束当前会话
/debug_reset  仅重置机器人 UI；不会关闭后台会话
```

会话元数据保存到 `~/.chatcli/sessions.json`。ChatCLI 重启时会检查 tmux 是否仍存在，恢复可用会话；已不存在的 tmux 会话会显示为已失效。
