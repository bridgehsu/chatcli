# ChatCLI 路线图

## 已完成

- Agent Session、Terminal Session、本地持久化与重启恢复。
- 状态驱动的意图识别 Profile。
- Shell 候选命令确认与安全策略。
- Channel 注册机制与 Telegram Channel。
- Session / Context Store 从 domain 拆至 infrastructure。
- `ChatOutput` 初步接入 Chat、System、Session Service。
- Shell 模型请求统一经 `infrastructure::llm_client`。

## 进行中

- Workspace、Shell、CLI、Terminal Service 脱离 Telegram `Bot`。
- 将 tmux 实时画面抽象为渠道无关的流式输出端口。
- 收敛 Agent 对 TerminalManager、WorkspaceTool、TerminalTool 的公开访问。

## 后续候选

- 真实 `ContextBuilder` 与历史摘要。
- Planner：处理多步骤复杂需求。
- 飞书、钉钉 Channel 实现。
- Session / Terminal Store 的可替换存储接口与更完整的集成测试。

