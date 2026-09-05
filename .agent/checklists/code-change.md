# 代码变更检查清单

## 开始前

- [ ] 已阅读根目录 `AGENTS.md` 与适用的 `docs/`。
- [ ] 已确定本次变更属于 Intent、Session、Service、Tool、Infrastructure 或 Channel 的哪一类。
- [ ] 已确认是否存在可复用的 Service、Tool、Store 或既有实现。
- [ ] 若涉及 Intent 或 Session 状态，已加载对应 Skill。

## 交付前

- [ ] 代码、测试及受影响的正式文档已同步更新。
- [ ] 已执行适用的人工回归用例；范围参见 `docs/engineering/testing.md`。
- [ ] 已执行 `cargo fmt` 与 `cargo test`。
- [ ] 已按 `CONTRIBUTING.md` 写好准确、聚焦的提交信息。
- [ ] 已记录实际验证结果、已知风险或未覆盖项。
