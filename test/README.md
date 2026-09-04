# ChatCLI 手工测试文档

本目录按用户可见的主要流程拆分测试文档。每次测试前，建议先执行：

```bash
cargo test
cargo run
```

Telegram Bot、模型路由和 tmux 必须可用；涉及 Shell 自然语言测试时，`router.enabled` 必须为 `true` 且 API Key 有效。

| 文档 | 覆盖流程 |
| --- | --- |
| [01-initial-and-reset.md](01-initial-and-reset.md) | 初始会话与 `/reset` |
| [02-agent-sessions.md](02-agent-sessions.md) | 创建、列表、切换 Agent Session |
| [03-workspace-and-cli.md](03-workspace-and-cli.md) | 目录选择与启动 CodeX / Cursor |
| [04-terminal-management.md](04-terminal-management.md) | Shell、终端列表、切换与关闭 |
| [05-shell-command-confirmation.md](05-shell-command-confirmation.md) | 自然语言生成 Shell 命令及确认 |
| [06-shell-safety-and-direct-input.md](06-shell-safety-and-direct-input.md) | 命令编辑、安全限制和直接输入 |
| [07-context-and-concurrency.md](07-context-and-concurrency.md) | 上下文隔离与同 chat 串行处理 |
| [08-tmux-health-check.md](08-tmux-health-check.md) | tmux 被手动关闭后的失效检测 |
| [09-auth-and-input-boundaries.md](09-auth-and-input-boundaries.md) | 授权、非文本和特殊输入边界 |
| [10-network-and-model-failures.md](10-network-and-model-failures.md) | Telegram、代理和模型异常降级 |
| [11-restart-and-persistence.md](11-restart-and-persistence.md) | 应用重启后的状态恢复 |
| [12-multi-user-isolation.md](12-multi-user-isolation.md) | 多 chat 的数据与终端隔离 |

测试过程中不要在真实工作目录中验证删除、覆盖或权限修改类命令。它们应被系统拒绝。
