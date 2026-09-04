# 测试约定

## 每次代码修改

```bash
cargo fmt
cargo test
```

## 人工测试入口

完整人工流程在 `test/README.md`，覆盖以下主题：

- 初始状态与重置
- Agent Session 管理
- 工作目录与 CLI
- 终端管理
- Shell 命令确认与安全
- 上下文、并发、重启恢复与多用户隔离
- Telegram、模型与网络失败

## 修改与回归范围

| 修改范围 | 至少执行 |
|---|---|
| Intent / YAML / Resolver | 单元测试 + `test/01`、`02` |
| Session / Context Store | 单元测试 + `test/02`、`07`、`11`、`12` |
| Workspace / CLI | 单元测试 + `test/03` |
| Terminal / tmux | 单元测试 + `test/04`、`08` |
| Shell | 单元测试 + `test/05`、`06` |
| Telegram / Output | 单元测试 + `test/09`、`10` |

