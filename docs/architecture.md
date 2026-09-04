# ChatCLI 架构与 Vibe Coding 约定

本文件是后续 AI 与开发者修改 ChatCLI 时的共同约定。目标是：功能可持续扩展，同时避免把 Telegram、tmux、文件存储、意图判断和业务流程混在同一个文件。

相关文档：

- `product-requirements.md`：产品边界与交互原则。
- `session-state-machine.md`：Session / Terminal 状态流转。
- `intent-and-actions.md`：Intent、Action 与新增意图检查项。
- `testing.md`：自动测试与人工回归入口。
- `roadmap.md`：已完成、进行中与后续事项。

## 核心模型

```text
一个 Chat
  └─ 多个 Agent Session
       └─ 多个 Terminal Session
            ├─ Shell
            ├─ CodeX
            └─ Cursor
```

- Agent Session：用户在聊天中的一段独立工作流程，有当前状态和历史上下文。
- Terminal Session：一个具体的 tmux 终端，属于一个 Agent Session。
- 每个 chat 始终应有一个当前 Agent Session；首次收到消息时自动创建 `Initial` Session。

## 目录职责

```text
src/
├── app/                    # 应用装配与一次用户交互的编排
├── service/                # 用例：会话、终端、目录、Shell、聊天等
├── agent/                  # 意图识别、策略、上下文、规划能力
├── domain/                 # 纯业务模型；不得依赖 app、Telegram、tmux、文件系统
├── tools/                  # Agent 可调用的原子能力
├── infrastructure/         # tmux、HTTP、LLM、本地文件持久化等技术实现
├── interfaces/             # Telegram、未来飞书/钉钉等外部渠道适配
└── shared/                 # 真正跨层的小型通用能力
```

### app

- `application.rs`：应用启动、全局依赖装配。
- `state.rs`：进程级共享依赖，例如 Config、Agent。
- `interaction.rs`：一轮完整用户交互：读取当前 Session → 意图识别 → 路由 → 执行 Action。
- `action_executor.rs`：将 `Decision.action` 分发至对应 Service。
- `agent_runtime.rs`：Agent 的运行时状态、恢复和对 Service 暴露的行为方法。

不要把具体 Telegram API、tmux 命令或 JSON 文件读写写进 `interaction.rs`。

### service

Service 按用例划分：

- `session_service`：Agent Session 创建、关闭、列表、切换。
- `terminal_service`：Terminal Session 创建、切换、关闭、转发。
- `workspace_service`：目录列表、目录解析与选择。
- `shell_service`：Shell 候选命令、校验、确认、取消、编辑。
- `chat_service`：Agent 普通聊天。
- `system_service`：帮助、介绍、重置等系统反馈。

新功能优先放到已有 Service；只有确实是新用例时再新建 `*_service.rs`。

Service 不应直接访问 Agent 的 `Mutex<HashMap<...>>` 字段；通过 Agent 的行为方法访问，例如：

```rust
state.selected_cli(chat).await;
state.set_pending_workspace(chat, pending).await;
state.take_pending_shell_command(chat).await;
```

新增消息反馈应优先使用 `service::output::ChatOutput`，不要把 Telegram Presenter 或 Telegram 键盘逻辑重新引回 Service。

### agent

- `intent/`：用户文本 → Intent；规则优先，模型兜底。
- `policy/`：安全限制，例如 Shell 命令白名单和拒绝规则。
- `context/`：未来负责模型上下文构建。
- `planner/`：未来复杂任务规划；P0/P1 不要把业务动作塞进这里。

意图层只负责“用户想做什么”，不直接操作 tmux、文件或 Telegram。

```text
文本 → Intent → Decision → ActionExecutor → Service → Tool / Infrastructure
```

确定性文本、斜杠命令和按钮回调都应先进入 Intent 规则；未命中才使用模型分类兜底。

### domain

`domain` 只能放稳定的业务数据和状态，例如：

```text
Session
SessionState
SessionStatus
PendingLaunch
PendingShellCommand
TerminalKind
AgentKind
```

禁止在 `domain` 中出现：

```text
Bot / ChatId（渠道 SDK 类型）
reqwest / tmux
fs::read / fs::write
Mutex
Config
AppState
```

### infrastructure

- `persistence/session_store.rs`：Session JSON 快照。
- `persistence/context_store.rs`：JSONL 上下文历史。
- `persistence/terminal_store.rs`：终端记录与 tmux 生命周期恢复。
- `llm_client.rs`：唯一的模型 HTTP 调用入口。
- `tmux_runner.rs`：tmux 命令封装。

任何新模型调用都应复用 `llm_client`，不要在 Service 中自行创建 `reqwest::Client`。

### interfaces

```text
interfaces/
├── channel.rs              # Channel 抽象与自注册定义
├── registry.rs             # 发现并启动已启用渠道
└── channels/
    └── telegram/           # Telegram 专属 adapter、client、output、view
```

新增飞书或钉钉时，新建 `interfaces/channels/feishu` 或 `dingtalk`，实现 `Channel` 并注册；不要修改业务 Service 来判断渠道类型。

## 依赖方向

只允许依赖向内：

```text
interfaces → app/service/agent/domain
app         → service/agent/domain/infrastructure
service     → agent/domain/tools/infrastructure
agent       → domain/infrastructure（仅模型配置与模型客户端）
tools       → domain/infrastructure
domain      → 无项目上层依赖
```

尤其禁止：

```text
domain → app
domain → interfaces
service → interfaces/channels/telegram
```

## 命名约定

- 一轮完整用户交互：`interaction.rs`，入口为 `interaction::run(...)`。
- “做什么”的业务用例：`*_service.rs`。
- “保存/读取什么”的技术实现：`*_store.rs`。
- “调用一个原子外部能力”：`tools/*`。
- “平台协议适配”：`interfaces/channels/<channel>`。
- 不要创建 `utils`、`common`、`helper` 这类无明确边界的目录。

## 修改前检查清单

1. 这是一种 Intent、Decision、Service、Tool 还是 Infrastructure 行为？
2. 新状态是否应进入 `domain`，并由 `persistence` 持久化？
3. 是否已经存在能复用的 Service、Tool、LlmClient 或 Store？
4. 是否把 Telegram、tmux、文件读写混进了 `domain` 或 `agent`？
5. 是否应通过 `ChatOutput` 回应用户，而不是直接依赖 Telegram？
6. 修改后运行：

```bash
cargo fmt
cargo test
```

## 文档维护规则

`docs/architecture.md` 是代码的一部分，不是一次性说明文档。

当一次修改涉及以下任意内容时，必须在同一个提交中同步更新本文件：

- 新增、删除、移动或重命名目录、模块、Service、Tool、Store、Channel。
- 改变模块职责、依赖方向或对外接口。
- 新增或删除领域模型、Session 状态、Terminal 状态、Intent、Decision、Action。
- 改变用户主流程、状态流转、持久化格式或渠道交互方式。
- 完成或替换“当前渐进式重构状态”中的事项。

仅修改实现细节、修复局部 Bug、补充测试且不改变上述结构时，不需要改本文档。

更新方式：修改对应章节；如果是重要结构变化，在本节末尾增加一条简短记录。

### 架构变更记录

- 2026-09-04：新增 `ChatOutput` 输出端口；Chat、System、Session Service 开始脱离 Telegram 输出实现。
- 2026-09-04：`agent_flow.rs` 重命名为 `interaction.rs`，表示一轮完整用户交互。
- 2026-09-04：Session 与 Context 持久化从 `domain` 拆分至 `infrastructure/persistence`。

## 当前渐进式重构状态

已完成：

- Session 模型与 Session/Context 持久化分离。
- `domain → app` 的 PendingLaunch 反向依赖已移除。
- Agent 的 CLI 选择、待目录、待 Shell 确认状态已收敛为行为方法。
- Chat、System、Session Service 已接入 `ChatOutput`。
- Telegram 作为 `interfaces/channels/telegram` 的可注册 Channel。

待继续：

- Workspace、Shell、CLI、Terminal Service 迁移至 `ChatOutput`。
- 将 tmux 终端实时输出抽象为渠道无关的流式输出端口。
- 继续收敛 `Agent` 对 TerminalManager、WorkspaceTool、TerminalTool 的直接暴露。
