# 多用户 / 多 Chat 隔离

## 前置条件

- 在 `telegram.allowed_user_ids` 中临时配置两个独立的测试 Telegram 用户。
- 两个用户分别记为 A 和 B。
- 两个用户都能私聊同一个 Bot。

## 步骤

1. A 发送 `/reset`，然后 `打开终端`。
2. B 发送 `/reset`，然后 `新建会话` 两次。
3. A 发送 `会话列表`、`终端列表`，记录会话与终端 ID。
4. B 发送 `会话列表`、`终端列表`，记录会话与终端 ID。
5. A 尝试发送 B 的会话或终端切换命令：

   ```text
   /agent_session <B的会话ID>
   /terminal <B的终端ID>
   ```

6. B 在自己的会话中和 Agent 聊天，A 询问与 B 对话无关的问题。
7. A 发送 `/reset`，再由 B 检查自己的会话和终端。

## 预期结果

- A 与 B 的会话列表、终端列表、当前终端和对话历史完全隔离。
- A 不能切换到 B 的 Agent Session 或 Terminal Session。
- A 的 `/reset` 只清除 A 的数据和受管理 tmux，不影响 B。
- 模型上下文不会混入另一 chat 的消息。

## 补充检查

查看 `~/.chatcli/agent-sessions.json` 和 `~/.chatcli/sessions.json` 时，应能看到不同 `chat_id` 的记录；测试中不要将其中的真实 Telegram ID、路径或模型内容上传到公开位置。
