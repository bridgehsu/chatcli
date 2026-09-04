# Intent、Decision 与 Action

## 链路

```text
用户文本 → Recognizer → Intent → Resolver → Decision.action → ActionExecutor → Service
```

规则优先；未命中规则时，依据当前 Session Profile 决定模型分类、Agent 聊天、目录输入、终端输入或 Shell 命令建议。

## 配置来源

意图目录位于 `config/intents.yaml`。新增配置意图时必须同时检查：

1. `config/intents.yaml` 的 `name`、`action`、描述和示例。
2. `IntentCatalog` 校验是否通过。
3. `Resolver` 是否能生成对应 `Decision`。
4. `app/action_executor.rs` 是否有对应 action 分发。
5. 目标 Service 与测试是否已实现。

## 当前主要映射

| Intent | Action | Service |
|---|---|---|
| `new_session` | `session.create` | `session_service` |
| `close_session` | `session.close` | `session_service` |
| `list_session` | `session.list` | `session_service` |
| `switch_session` | `session.switch` | `session_service` |
| `choose_cli` | `cli.choose` | `cli_service` |
| `open_terminal` | `terminal.open` | `terminal_service` |
| `list_directories` | `workspace.list` | `workspace_service` |
| `shell_confirm` | `shell.confirm` | `shell_service` |
| `reset` | `session.reset` | `system_service` |
| `agent_chat` | `agent.chat` | `chat_service` |

## 特殊输入

- `/agent_session <id>`：切换 Agent Session。
- `/terminal <id>`：切换 Terminal Session。
- `/reset`：清空当前 chat 的受管理数据。
- `$<command>` 或 `!<command>`：明确直接输入 Shell，不走模型候选。
- `修改为：<command>`：仅在等待 Shell 确认时编辑候选命令。

