# 路线图

导航：[文档首页](README.md) · [架构](design/architecture.md)

---

## 已完成

| 项 |
|---|
| Agent Session、Terminal Session、本地持久化与重启恢复 |
| 状态驱动的意图识别 Profile |
| Shell 候选命令确认与安全策略 |
| Channel 注册机制与 Telegram Channel |
| Session / Context Store 从 domain 拆到 infrastructure |
| `ChatOutput` 初步接入 Chat · System · Session Service |
| Shell 模型请求统一经 `infrastructure::llm_client` |

---

## 进行中

| 项 |
|---|
| Workspace · Shell · CLI · Terminal Service 脱离 Telegram `Bot` |
| tmux 实时画面抽象为渠道无关流式输出端口 |
| 收敛 Agent 对 TerminalManager · WorkspaceTool · TerminalTool 的公开访问 |
| 将当前 `SessionState` 收敛为以 `TerminalKind` 驱动的交互状态；统一 Shell、CodeX、Cursor 的目录选择流程 |
| 使用 FSM + Reducer + Effect 取代“运行时数据反推 SessionState”的状态持久化方式 |

---

## 后续候选

| 项 |
|---|
| 真实 `ContextBuilder` 与历史摘要 |
| Planner：多步骤复杂需求 |
| 飞书 · 钉钉 Channel |
| Session / Terminal Store 可替换存储接口与更完整集成测试 |
