//! Minimal deterministic intent recognition for the first demo.

pub mod commands;
mod model;
mod normalize;

use crate::agent::AgentKind;
use crate::infrastructure::config::RouterConfig;
use normalize::prepare;
use serde::Deserialize;
use std::sync::Arc;
use strum::{Display, EnumIter, IntoEnumIterator};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntentKind {
    ChooseCli(AgentKind),
    OpenTerminal,
    SelectWorkspace,
    ListDirectories,
    CloseTerminal,
    Unknown,
}

/// 模型分类接口使用的无参数意图名称。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Display, EnumIter)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum IntentName {
    ChooseCli,
    OpenTerminal,
    SelectWorkspace,
    ListDirectories,
    CloseTerminal,
    Unknown,
}

impl IntentName {
    pub fn classifier_values() -> String {
        Self::iter()
            .map(|intent| intent.to_string())
            .collect::<Vec<_>>()
            .join("|")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Intent {
    pub kind: IntentKind,
}

/// 意图识别器在系统启动时创建，并持有模型路由配置。
pub struct Recognizer {
    router: Arc<RouterConfig>,
}

impl Recognizer {
    pub fn new(router: Arc<RouterConfig>) -> Self {
        Self { router }
    }

    /// 统一识别入口：精确规则优先，未命中时才走模型兜底。
    pub async fn run(&self, input: &str) -> Intent {
        let input = prepare(input);
        let kind = match input.lower.as_str() {
            "打开终端" | "开启终端" | "打开" => Some(IntentKind::OpenTerminal),
            "关闭终端" | "结束终端" | "关闭" => Some(IntentKind::CloseTerminal),
            "codex" => Some(IntentKind::ChooseCli(AgentKind::Codex)),
            "cursor" => Some(IntentKind::ChooseCli(AgentKind::Cursor)),
            _ => None,
        };
        if let Some(kind) = kind {
            return Intent { kind };
        }
        model::recognize(&input, &self.router)
            .await
            .unwrap_or(Intent {
                kind: IntentKind::Unknown,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn recognizes_open_terminal() {
        assert_eq!(
            Recognizer::new(Arc::new(RouterConfig::default()))
                .run("打开终端")
                .await
                .kind,
            IntentKind::OpenTerminal
        );
    }

    #[tokio::test]
    async fn keeps_unmatched_text_unknown() {
        assert_eq!(
            Recognizer::new(Arc::new(RouterConfig::default()))
                .run("帮我修复测试")
                .await
                .kind,
            IntentKind::Unknown
        );
    }
}
