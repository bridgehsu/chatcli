# ChatCLI 开发约定

改代码前先读：

1. [docs/README.md](docs/README.md) — 文档地图
2. [docs/design/architecture.md](docs/design/architecture.md) · [docs/engineering/standards.md](docs/engineering/standards.md)

按任务再读：`product` · `design/interaction` · `design/sessions` · `design/intents` · `engineering/testing` · `roadmap`。

触及目录职责、依赖、模型、状态、渠道、工具边界或重构进度时，**同一变更里同步更新对应 `docs/`**。

AI 协作资产：

- 通用工作规则与检查清单见 [`.agent/`](.agent/README.md)。
- 新增 Intent 或变更 Session 状态时，先读对应的 `.agent/skills/` 操作手册。
- `.agent/` 不替代 `docs/`；产品、架构和工程规范始终以 `docs/` 为准。

提交前：

```bash
cargo fmt
cargo test
```

分支、提交信息与 Pull Request 约定见 [`CONTRIBUTING.md`](CONTRIBUTING.md)。
