# Runbooks

可重复执行的运行、部署与排障流程。

## 建议收录

| 主题 | 示例 |
|---|---|
| 网络 | Telegram 代理配置与连通性检查 |
| 终端 | tmux 会话丢失后的恢复步骤 |
| 模型 | DeepSeek / OpenAI 兼容端点连不上时的排查 |

## Telegram 单实例

同一个 Telegram Bot 只能有一个 `getUpdates` long-polling 消费者。若日志出现 `TerminatedByOtherGetUpdates`，停止另一台机器、旧的 `cargo run` 进程或其他部署后，再执行 `./bin/cmd.sh restart`。

launchd 不继承交互式终端的 `PATH`。安装脚本使用的 `com.chatcli.plist` 已包含 Homebrew、`/usr/local/bin` 和 ChatGPT.app 内置 Codex 的常见路径；若 CLI 安装在其他位置，应在该 plist 的 `PATH` 中补充其目录后重新安装。

导航：[文档首页](../README.md) · [测试](../engineering/testing.md)
