# ChatCLI 协作约定

本文件是分支、提交与 Pull Request 约定的唯一来源。项目行为、架构和工程规则请阅读 [`docs/`](docs/README.md)；AI 的任务流程请从 [`AGENTS.md`](AGENTS.md) 开始。

## 开始前

1. 阅读 `AGENTS.md` 和适用的正式文档。
2. 确认改动范围；一项提交应只聚焦一个可独立理解、验证和回滚的目的。
3. 不提交密钥、Token、真实配置、用户数据或构建产物。

## 分支

使用简短、可读的名称；推荐形式：

```text
feat/<short-description>
fix/<short-description>
docs/<short-description>
refactor/<short-description>
```

不要直接在受保护的主分支上改写历史，也不要对主分支使用强制推送。

## 提交信息

使用以下格式：

```text
<type>(<scope>): <imperative summary>
```

`scope` 可省略；`summary` 简短说明此次变更的结果。可用的 `type`：

| Type | 用途 |
|---|---|
| `feat` | 新增用户可见能力 |
| `fix` | 修复缺陷 |
| `refactor` | 不改变外部行为的代码调整 |
| `docs` | 仅文档或协作资产变更 |
| `test` | 仅测试变更 |
| `chore` | 构建、依赖或工具维护 |

示例：

```text
feat(session): add terminal kind selection
fix(shell): reject unsafe edited command
refactor(output): decouple workspace service from telegram
docs(agent): add intent change skill
test(intent): cover ambiguous workspace request
```

## 验证与文档

代码提交前至少执行：

```bash
cargo fmt
cargo test
```

按 [`docs/engineering/testing.md`](docs/engineering/testing.md) 运行与改动范围匹配的人工回归。若改动影响产品行为、架构边界、状态机、Intent、渠道或运行方式，必须在同一提交中更新对应 `docs/`；重要长期架构决策应新增 ADR。

## Pull Request

PR 描述至少回答：

```md
## 改了什么
## 为什么这样改
## 如何验证
```

列出实际执行的命令、人工回归结果，以及已知风险或未覆盖项。不要用“已测试”代替具体证据。
