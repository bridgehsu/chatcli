# Session 与 Terminal 状态机

## 层级

```text
Chat → Agent Session → Terminal Session
```

一个 Chat 有多个 Agent Session；每个 Agent Session 有多个 Terminal Session，但同一时刻最多选择一个当前 Terminal。

## Agent SessionState

| 状态 | 含义 | 普通文本去向 |
|---|---|---|
| `Initial` | 新会话，未选择 CLI、未选终端 | Agent 聊天 / 模型意图兜底 |
| `CliSelected` | 已选择 CodeX 或 Cursor | 等待或请求目录 |
| `WaitingWorkspace` | 正在选择工作目录 | 目录序号、路径、目录名 |
| `ActiveTerminal` | 有当前 tmux 终端 | Shell 请求或直接转发 CLI |
| `WaitingShellCommand` | Shell 候选命令待确认 | 仅确认、取消、修改命令与受限控制操作 |

## 主要流转

```text
Initial → ChooseCli → WaitingWorkspace → ActiveTerminal(CodeX / Cursor)
Initial → OpenTerminal → ActiveTerminal(Shell)
ActiveTerminal(Shell) → ShellRequest → WaitingShellCommand
WaitingShellCommand → Confirm / Cancel / Edit → ActiveTerminal(Shell)
任意状态 → NewSession → Initial
任意状态 → Reset → 清空数据后创建 Initial
```

## 不变量

- 每个 chat 必须可恢复一个当前 Agent Session；不存在时自动创建。
- `WaitingWorkspace` 优先消费普通文本，防止目录输入误发至终端。
- `WaitingShellCommand` 不允许普通文本直接执行或转发。
- 切换 Session 时恢复其目录候选、CLI 选择、待确认 Shell 命令及当前终端。
- `/reset` 清除该 chat 的 Session、上下文及受管理终端，然后新建 Initial Session。

