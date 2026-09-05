# ChatCLI 文档地图

| 文档 | 何时阅读 |
|---|---|
| [design/architecture.md](design/architecture.md) | 调整目录、模块职责或依赖方向 |
| [product.md](product.md) | 设计或扩展产品能力 |
| [design/interaction.md](design/interaction.md) | 调整一条消息的主流程、路由或渠道入口 |
| [design/sessions.md](design/sessions.md) | 调整 Session、Terminal、状态转换 |
| [design/intents.md](design/intents.md) | 新增 Intent、Action、规则或 Profile |
| [engineering/standards.md](engineering/standards.md) | 修改 Rust 代码前必读 |
| [engineering/testing.md](engineering/testing.md) | 编写或执行回归测试 |
| [roadmap.md](roadmap.md) | 规划后续工作 |

补充目录：

- [adr/](adr/)：重要架构决策记录。
- [operations/](operations/)：运行与故障排查手册。

## 文档约定

- `docs/` 记录项目事实：产品行为、设计结论、工程规则、运行方式和架构决策。
- `.agent/` 记录 AI 协作资产：任务步骤、检查清单和文档模板；它不复制或覆盖 `docs/` 的正式内容。
- 改动实现时，若影响某份文档所定义的行为或边界，必须同一变更更新该文档。
