# 授权与输入边界

## 目标

验证未授权用户、非文本消息和特殊文本不会执行 Agent 或终端操作。

## A. 未授权用户

### 前置条件

- 准备一个 Telegram ID 不在 `config.yaml` 的 `telegram.allowed_user_ids` 中的账号。
- 主测试账号保持在允许列表中。

### 步骤

1. 用未授权账号向 Bot 发送 `打开终端`。
2. 用主测试账号发送 `会话列表` 和 `终端列表`。
3. 查看应用日志。

### 预期结果

- 未授权账号不收到业务回复。
- 日志出现 `Rejected unauthorized Telegram message`。
- 主测试账号没有新增 Agent Session 或 tmux 终端。

## B. 非文本和空输入

### 步骤

1. 向 Bot 发送一张图片、语音、贴纸或文件。
2. 发送仅包含空格的文本。
3. 再发送 `帮助`。

### 预期结果

- 非文本和空文本被忽略，日志出现 `Ignored non-text or empty Telegram update`。
- Bot 服务不中断，后续“帮助”仍能正常响应。

## C. 特殊字符与超长输入

### 步骤

1. 在初始会话发送包含 Markdown、HTML、emoji 和反引号的文本，例如：

   ```text
   <b>测试</b> `code` 😀
   ```

2. 在 Shell 中发送：

   ```text
   查看包含 <test> 的文件
   ```

3. 发送一条超过 4,000 字符的普通文本。

### 预期结果

- Telegram 回复不会出现 HTML 注入或渲染异常。
- Shell 候选命令仍需要确认，特殊字符不会直接进入 tmux。
- 超长输入失败时应有明确提示或安全降级，服务不能崩溃。
