# 代码规范

导航：[文档首页](../README.md) · [架构](../design/architecture.md) · [测试](testing.md)

---

## Rust

| 规则 | 说明 |
|---|---|
| 提交前必跑 | `cargo fmt` · `cargo test` |
| 注释 | 公开类型、状态、关键流程用简明中文 |
| 错误处理 | 用户输入 / 文件 / 网络 / 外部命令避免 `unwrap()` |
| 日志 | 不输出 Token、API Key、完整敏感路径、原始认证信息 |

---

## 分层硬规则

| 层 | 约束 |
|---|---|
| `domain` | 不依赖 `app`、`interfaces`、Telegram、tmux、文件系统、HTTP |
| Service | 不直接访问 Agent 内部 `Mutex` / `HashMap`；走行为方法 |
| 模型调用 | 只能经 `infrastructure::llm_client` |
| 持久化 | 只能放 `infrastructure::persistence` |
| 新渠道 | `interfaces/channels/<channel>` + 实现 `Channel` |
| 用户输出 | 优先 `service::output::ChatOutput`，新 Service 不要直接依赖 Telegram |

---

## 新功能同步

| 变更 | 必须同步 |
|---|---|
| 新 Intent | YAML · Resolver · ActionExecutor · Service · 测试 · `design/intents.md` |
| 新 Session 状态 | domain · 持久化恢复 · 识别 Profile · 状态机文档 · 测试 |
| 架构变更 | `design/architecture.md` · `roadmap.md` |

## 协作与提交

分支命名、提交信息、提交前验证与 Pull Request 要求以根目录 [`CONTRIBUTING.md`](../../CONTRIBUTING.md) 为准。
