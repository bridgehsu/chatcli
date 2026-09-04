# tmux 终端健康检查

## 步骤

1. 在 Telegram 中发送 `/reset`，再发送 `打开终端`。
2. 在本机终端执行：

   ```bash
   tmux list-sessions
   ```

3. 找到名称以 `chatcli-tg-` 开头的会话，然后执行：

   ```bash
   tmux kill-session -t <tmux会话名>
   ```

4. 回到 Telegram，发送 `终端列表`。
5. 再发送一条 Shell 自然语言请求或 `$ pwd`。

## 预期结果

- ChatCLI 检测到 tmux 会话已不存在后，将该终端标记为“已失效”。
- 已失效终端不会继续作为当前可发送目标。
- 后续发送输入时，机器人提示当前没有可用终端或目标终端不可用，而不是静默成功。
