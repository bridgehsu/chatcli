# AI 协作资产

项目正式文档位于 [`docs/`](../docs/)；AI 修改代码前请先从根目录的 [`AGENTS.md`](../AGENTS.md) 读取适用约定。

| 目录 | 用途 |
|---|---|
| [`rules/`](rules/README.md) | AI 工作方式、风险边界等 AI 专属规则 |
| [`skills/`](skills/README.md) | 反复出现的项目任务操作手册 |
| [`checklists/`](checklists/README.md) | 开始或完成任务时的核对清单 |
| [`templates/`](templates/README.md) | Feature Spec、ADR 等文档骨架 |
| [`prompts/`](prompts/README.md) | 可跨会话复用的任务提示词 |
| [`mcp/`](mcp/README.md) | 工具使用说明、权限边界与无密钥示例 |

`.agent/` 不复制正式项目文档，避免两份文档逐渐不一致。产品、架构、工程规则和运行说明以 [`docs/`](../docs/README.md) 为唯一事实来源。
