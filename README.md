# ChatCLI

ChatCLI 把聊天消息接到本机持久化 `tmux`。它只做展示与输入转发；Codex / Cursor 仍在你自己的 Mac 上真实运行。

```mermaid
flowchart LR
  Telegram --> ChatCLI --> tmux --> CLI[Codex / Cursor / Shell]
```

完整文档见 **[docs/](docs/README.md)**。

| 快速入口 | |
|---|---|
| [架构](docs/design/architecture.md) | [产品范围](docs/product.md) |
| [主交互流程](docs/design/interaction.md) | [状态机](docs/design/sessions.md) |
| [Intent / Action](docs/design/intents.md) | [代码规范](docs/engineering/standards.md) |
| [测试](docs/engineering/testing.md) | [路线图](docs/roadmap.md) |

协作、分支、提交与 Pull Request 约定见 [CONTRIBUTING.md](CONTRIBUTING.md)。

消息先经 `Agent`：确定性命令与会话状态优先按规则处理；歧义自然语言可走 `router` 模型意图分类，失败则回退启发式。`Agent` 不直接执行 shell，也不编造路径；终端与文件系统结果一律来自本机模块。

## Agent 与 Tool

`Agent` 只产出受限决策；本机能力在 `src/tools/`：

| Tool | 能力 |
|---|---|
| `WorkspaceSearchTool` | 查目录、校验路径、定位文件 |
| `TerminalTool` | 启动 CLI、输入、抓屏、按键、本机接管 |

例如选 Codex 后发「打开 chatcli 这个目录」：唯一命中直接建 tmux；多个命中给编号候选；无结果提示重输。路径来自真实文件系统。

一期：macOS · Telegram · Codex · Cursor。每个聊天可有多个 tmux，但任意时刻只有一个当前会话；启动前先选工作目录（不必是 Git 仓库）。

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
