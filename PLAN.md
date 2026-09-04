# ChatCLI

ChatCLI 是一个通过 Telegram、飞书和本机 Terminal 控制同一真实终端的工具。

它不是聊天版 AI Agent，也不负责模型编排。它只把聊天消息和终端屏幕连接到持久化的 `tmux` 会话；Codex、Cursor、`git`、`npm`、`ssh` 等程序仍在用户自己的电脑上真实运行。

```text
Telegram / 飞书 / 本机 Terminal
              ↓
          ChatCLI 服务
              ↓
     TerminalSessionManager
              ↓
          tmux + zsh
              ↓
        Codex / Cursor
```

## 产品目标

用户不在电脑旁时，仍可通过聊天软件：

- 启动或恢复本机 Codex 或 Cursor 终端；
- 向当前交互式 CLI 输入文字；
- 查看终端输出、处理中断和确认提示；
- 回到电脑后接入同一个 tmux 会话继续操作。

终端是唯一事实来源。聊天软件只是远程展示与控制入口。

## 一期范围

一期只支持：

- Telegram；
- 本机 macOS；
- Codex CLI；
- Cursor CLI；
- `tmux` 持久化会话；
- 每个聊天一个当前终端；
- 终端抓屏、输入转发、`Ctrl+C`、本机 attach 与关闭会话。

**暂不接入 Claude。** Claude 相关 profile、按钮和配置不属于一期需求。

飞书是二期渠道适配器；它必须复用同一个终端会话层，而不是实现另一套终端逻辑。

## 核心闭环

```text
1. Mac 开机，ChatCLI 常驻运行。
2. 用户在 Telegram 点 Open。
3. 用户选择 Codex 或 Cursor，并选择或输入启动目录。
4. ChatCLI 在该目录创建 tmux session。
5. 聊天消息输入真实 tmux 终端。
6. tmux 的终端输出回传到 Telegram。
7. 用户在手机查看屏幕、发送输入、处理中断或确认。
8. 用户回到电脑后执行 tmux attach，接管同一会话。
9. 用户关闭终端，ChatCLI 销毁对应 tmux session。
```

## 终端与会话模型

一个 Telegram 私聊或 topic 对应一个 `TerminalSession`：

```text
Telegram chat/topic
      ↓
TerminalSession
├─ tmux session：chatcli-tg-<chat-id>
├─ 工作目录：用户在启动时选择
├─ 当前模式：codex / cursor
└─ 当前屏幕与滚动日志
```

一期不要求用户预先配置项目目录，也不要求目录是 Git 仓库。启动 Codex 或 Cursor 前，ChatCLI 会要求用户选择目录：可以点选 Home (`~`)，或发送绝对路径、相对 Home 的路径、`~/项目目录`。

```text
用户：/cli codex
ChatCLI：请发送工作目录，例如 ~/demo-workspace/chatcli
用户：~/demo-workspace/chatcli
用户：修复当前项目的测试
```

## CLI Profile

ChatCLI 只保存启动命令，不理解模型任务内容：

```text
codex   → codex
cursor  → cursor agent
```

`/cli codex` 与 `/cli cursor` 会创建真实 tmux 终端并直接启动对应 CLI。后续聊天消息直接输入该交互式 CLI。

后续可以为 Codex 与 Cursor增加可选 profile 参数，例如模型、sandbox 或其他 CLI 参数；这仍是“启动配置”，不是 ChatCLI 自己的模型路由能力。

## 聊天交互层级

Telegram UI 必须根据终端状态变化，不应长期显示所有按钮。

### 无活动终端

```text
当前没有活动终端。

[Open]
```

### 点击 Open

```text
选择要启动的 CLI：

[Codex] [Cursor]
[取消]
```

### 当前为 Codex 或 Cursor

```text
终端 #1 · Codex · ~

[查看屏幕] [电脑接管]
[中断当前任务] [关闭终端]
```

处于 Codex 或 Cursor 时，不显示另一个 CLI 的“切换”按钮，避免误杀当前会话。当前一期为每个聊天保留一个终端；要启动另一个 CLI，先关闭当前终端，再打开新的终端。

优先使用 Telegram Inline Keyboard：按钮绑定在当前终端状态消息上，状态变化时编辑同一条消息；聊天输入框始终用于输入真实终端。

## 终端展示与操作

ChatCLI 不做远程桌面。它提供面向手机的终端视图：

```text
默认实时视图：最近 20～40 行，更新同一条 Telegram 消息
/screen：当前终端屏幕
/log：更多滚动历史
/attach：本机 Terminal attach 到同一 tmux
/stop：向终端发送 Ctrl+C
/close：关闭 tmux session
```

终端输出经过 ANSI 清理与截断后显示，但真实 tmux 屏幕始终保留。以后可以识别常见交互提示：

```text
[Y/n] / Allow-Deny / 1-2-3
        ↓
Telegram Inline Buttons
        ↓
tmux send-keys 注入真实按键
```

## 架构职责

```text
channels/
  telegram/       消息、按钮与展示
  feishu/         二期：复用相同接口

terminal/
  session_manager 聊天 ↔ tmux 映射、恢复与关闭
  tmux_backend    创建、输入、抓屏、attach、按键注入
  screen_renderer ANSI 清理、终端屏幕视图

profiles/
  codex           Codex 启动命令
  cursor          Cursor 启动命令

app/
  router          确定性命令路由与生命周期
```

## 明确不做

以下内容不属于一期：

- 自研 AI Agent；
- 普通模型聊天或知识问答；
- `codex exec --json` 作为默认执行路径；
- 多模型自动路由；
- 通过模型做二级意图识别；
- 强制项目目录或 Git 仓库起点；
- Shell 作为独立的聊天终端模式；
- 以 CLI session ID 取代 tmux 作为会话事实来源；
- Claude 集成；
- 多人账户体系、复杂权限系统与云端任务编排。

一期只使用确定性规则：

```text
/open
/cli codex
/cli cursor
/screen
/attach
/stop
/close
```

没有终端时，普通文本提示用户 Open；有终端时，普通文本直接输入当前 tmux。

## 后续路线

1. 稳定 Telegram + tmux + Codex/Cursor 闭环。
2. 增加服务重启后的 `chat_id ↔ tmux session` 恢复。
3. 增加 `y/n`、Allow/Deny、编号选项的按钮审批。
4. 增加飞书 Adapter。
5. 增加 topic/thread 多终端、多个本机或服务器选择。

## 运行前提

- Mac 保持开机且避免深度睡眠；
- ChatCLI 以 launchd 常驻；
- `tmux` 已安装；
- Codex、Cursor 已在同一 macOS 用户下登录；
- Telegram 用户 ID 位于本机白名单中；
- Telegram 网络与本地代理可用。

## 本地构建与常驻运行

先复制并填写本机配置；`config.yaml` 含 Telegram token，不会提交到 Git：

```bash
cp config.example.yaml config.yaml
```

日常运维使用 `bin/cmd.sh`：

```bash
./bin/cmd.sh build       # 格式检查、测试、静态检查、release 构建
./bin/cmd.sh run         # 前台本地运行
./bin/cmd.sh install     # 安装并启动当前登录会话中的服务
./bin/cmd.sh start       # 启动已安装的服务
./bin/cmd.sh stop        # 停止服务
./bin/cmd.sh restart     # 重启服务
./bin/cmd.sh status      # 查看 launchd 服务状态
./bin/cmd.sh logs        # 持续查看 launchd 日志
./bin/cmd.sh uninstall   # 停止并卸载 launchd 服务
./bin/cmd.sh help        # 查看全部命令
```

`bin/cmd.sh` 只负责命令分发：`scripts/` 存放构建、安装与卸载脚本，`bin/` 存放运行、启停和日常运维命令。`install` 使用当前项目目录中的 release 二进制、`config.yaml` 与 `logs/`，不会覆盖已有配置文件。

默认不会配置登录自启动或崩溃自动重启。`install` 只会在执行当次启动服务；用户如需自启动，可自行编辑 `~/Library/LaunchAgents/com.chatcli.plist` 并添加 `RunAtLoad`、`KeepAlive` 等 launchd 配置。
