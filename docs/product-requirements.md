# ChatCLI 产品范围

## 一句话定位

ChatCLI 通过聊天渠道管理本机的 Shell、CodeX、Cursor 与 tmux 会话。

## P0 已支持

- Agent Session 的创建、关闭、列表、切换与 `/reset`。
- 选择工作目录后启动 CodeX 或 Cursor。
- 创建、查看、切换、关闭 tmux Shell / CodeX / Cursor 终端。
- 将普通文本转发到当前终端。
- Shell 自然语言请求生成候选命令，用户确认后执行。
- Agent 普通聊天、帮助与自我介绍。
- 本地 JSON / JSONL 持久化 Session、终端与上下文。

## 非目标

- 不自动执行模型生成的 Shell 命令。
- 不把“等待目录”时的文本转发到旧终端。
- 不引入数据库作为当前阶段的持久化方案。
- 不支持未经授权的 Telegram 用户。
- 不把 ChatCLI 定位为多人协作或云端 IDE。

## 交互原则

- 用户主要在聊天框输入操作，不依赖 Telegram 底部键盘。
- 明确控制语句优先于普通文本和模型兜底。
- 无当前终端时，普通文本可以与 Agent 聊天。
- 当前为 Shell 时，自然语言默认生成待确认命令；`$` 或 `!` 前缀代表直接 Shell 输入。

