# 变更 Session 状态

## 触发条件

新增、删除或修改 `SessionState`、状态转换、持久化恢复或识别 Profile 时使用。

## 先读

1. [`docs/design/sessions.md`](../../../docs/design/sessions.md)
2. [`docs/design/architecture.md`](../../../docs/design/architecture.md)
3. [`docs/engineering/standards.md`](../../../docs/engineering/standards.md)
4. [`docs/engineering/testing.md`](../../../docs/engineering/testing.md)

## 流程

1. 写清状态的进入条件、允许迁移、退出条件、取消路径和重启恢复语义。
2. 将稳定状态模型放在 `domain`；持久化放在 `infrastructure::persistence`，不引入反向依赖。
3. 更新状态识别 Profile、交互编排及受影响的 Service；渠道适配不应泄漏到领域模型。
4. 覆盖正常迁移、取消、异常输入和重启恢复的测试。
5. 同步更新 `docs/design/sessions.md`；涉及主流程或结构边界时更新对应设计文档。
6. 按测试规范运行人工回归，并执行 `cargo fmt`、`cargo test`。

## 完成标准

- 状态转换可从文档和测试中独立理解。
- 重启恢复与运行时状态语义一致。
- `domain` 不依赖渠道、tmux、文件系统或 HTTP。
