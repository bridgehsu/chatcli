# 测试约定

导航：[文档首页](../README.md) · [代码规范](standards.md) · [人工用例](../../test/README.md)

---

## 每次改代码

```bash
cargo fmt
cargo test
```

---

## 人工回归

完整流程见 [`test/README.md`](../../test/README.md)，覆盖：

- 初始状态与重置
- Agent Session 管理
- 工作目录与 CLI
- 终端管理
- Shell 确认与安全
- 上下文、并发、重启恢复、多用户隔离
- Telegram / 模型 / 网络失败

---

## 修改 → 至少跑哪些

| 修改范围 | 单元测试之外 |
|---|---|
| Intent / YAML / Resolver | `test/01` · `02` |
| Session / Context Store | `test/02` · `07` · `11` · `12` |
| Workspace / CLI | `test/03` |
| Terminal / tmux | `test/04` · `08` |
| Shell | `test/05` · `06` |
| Telegram / Output | `test/09` · `10` |
