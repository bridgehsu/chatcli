# ChatCLI 代码规范

## Rust 基础规则

- 提交前必须执行 `cargo fmt` 与 `cargo test`。
- 新增公开类型、状态与关键流程必须有简明中文注释。
- 避免 `unwrap()` 处理用户输入、文件、网络或外部命令结果。
- 日志不输出 Token、API Key、完整敏感路径或原始认证信息。

## 分层规则

- `domain` 不依赖 `app`、`interfaces`、Telegram、tmux、文件系统或 HTTP。
- Service 不直接访问 Agent 内部 `Mutex` / `HashMap`；使用 Agent 暴露的行为方法。
- 模型 HTTP 请求只能经 `infrastructure::llm_client`。
- 本地持久化只能放入 `infrastructure::persistence`。
- 新渠道放入 `interfaces/channels/<channel>` 并实现 `Channel`。
- 用户输出优先经 `service::output::ChatOutput`，不要让新的 Service 直接依赖 Telegram。

## 新功能检查

- 新 Intent：同步更新 YAML、Resolver、ActionExecutor、Service、测试与 `docs/intent-and-actions.md`。
- 新 Session 状态：同步更新 domain、持久化恢复、识别 Profile、状态机文档与测试。
- 架构变更：同步维护 `docs/architecture.md` 与 `docs/roadmap.md`。
