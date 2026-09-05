# 新增或调整 Intent

## 触发条件

新增、删除或修改 Intent、Action、Resolver、规则匹配或 Profile 时使用。

## 先读

1. [`docs/design/intents.md`](../../../docs/design/intents.md)
2. [`docs/design/interaction.md`](../../../docs/design/interaction.md)
3. [`docs/engineering/standards.md`](../../../docs/engineering/standards.md)
4. [`docs/engineering/testing.md`](../../../docs/engineering/testing.md)

## 流程

1. 定义输入、Intent、Decision / Action、优先级和未知输入的回退行为。
2. 确认该行为只负责识别意图；真实业务动作放在 Service / Tool 边界之后。
3. 同步 YAML、Resolver、ActionExecutor、对应 Service 与测试；只修改实际受影响的部分。
4. 更新 `docs/design/intents.md`；若主交互或渠道入口变化，再更新 `interaction.md`。
5. 按测试规范执行对应的人工回归用例，并运行 `cargo fmt`、`cargo test`。

## 完成标准

- 新旧输入的匹配优先级明确，未知输入不会误触发危险动作。
- 规则、测试和正式文档描述同一行为。
