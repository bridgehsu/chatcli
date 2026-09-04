# ChatCLI 开发约定

修改代码前，先阅读 [docs/architecture.md](docs/architecture.md) 与 [docs/coding-standards.md](docs/coding-standards.md)，再按任务需要阅读 `docs/product-requirements.md`、`docs/session-state-machine.md`、`docs/intent-and-actions.md`、`docs/testing.md` 与 `docs/roadmap.md`。

如果本次修改影响目录职责、模块依赖、核心模型、状态流转、渠道接入、工具边界、命名约定或重构进度，必须在同一个变更中同步更新对应 `docs/` 文档。

完成修改前必须执行：

```bash
cargo fmt
cargo test
```
