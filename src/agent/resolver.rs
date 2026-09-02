//! Agent 的决策转换层。
//!
//! Resolver 将“确定性命令”或“意图层已经识别出的 Intent”转换为
//! `Decision`。它不读取会话状态、不操作 tmux，也不执行文件系统工具。

use crate::infrastructure::config::RouterConfig;

use crate::agent::intent::commands::Command;
use crate::agent::intent::IntentKind;

/// Agent 可以执行的下一步会话动作。
#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    /// 执行按钮或斜杠命令等确定性控制操作。
    Control(Command),
    /// 尚未明确选择 CLI 时，展示 Codex / Cursor 选择页。
    OfferCli,
    SelectWorkspace,
    ListDirectories,
    /// 意图层与模型均无法可靠识别用户输入。
    Unknown,
}

/// 将文本和意图识别结果转换为会话决策。
///
/// 当前没有持有运行时资源；保留为结构体是为了后续可以加入决策策略配置。
pub struct Resolver {}

impl Resolver {
    pub fn new(router: &RouterConfig) -> anyhow::Result<Self> {
        // 模型调用已经下沉到 intent/model.rs；此处保留参数以保持 Agent 的
        // 初始化接口稳定，并为后续 Resolver 策略配置预留位置。
        let _ = router;
        Ok(Self {})
    }

    pub fn decide(&self, rule_intent: crate::agent::intent::Intent) -> Decision {
        // Resolver 只处理意图层已经产生的分类结果。
        rule_to_decision(rule_intent)
    }
}

fn rule_to_decision(intent: crate::agent::intent::Intent) -> Decision {
    // 这里只做“意图 → 可执行决策”的映射；实际创建、关闭或展示页面仍由 Agent 执行。
    match intent.kind {
        IntentKind::OpenTerminal => Decision::OfferCli,
        IntentKind::CloseTerminal => Decision::Control(Command::Close),
        IntentKind::ChooseCli(kind) => Decision::Control(Command::NewSession(kind)),
        IntentKind::SelectWorkspace => Decision::SelectWorkspace,
        IntentKind::ListDirectories => Decision::ListDirectories,
        IntentKind::Unknown => Decision::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent() -> Resolver {
        Resolver::new(&RouterConfig::default()).unwrap()
    }

    #[test]
    fn recognizes_close_terminal() {
        let decision = agent().decide(crate::agent::intent::Intent {
            kind: IntentKind::CloseTerminal,
        });
        assert_eq!(decision, Decision::Control(Command::Close));
    }

    #[test]
    fn preserves_unknown_intent() {
        let decision = agent().decide(crate::agent::intent::Intent {
            kind: IntentKind::Unknown,
        });
        assert_eq!(decision, Decision::Unknown);
    }
}
