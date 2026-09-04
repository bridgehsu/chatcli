//! 将配置意图转换为动作请求；不持有业务状态，也不直接执行工具。

use std::sync::Arc;

use crate::{
    agent::{
        intent::{Intent, IntentCatalog},
        AgentKind,
    },
    infrastructure::config::RouterConfig,
};

/// 配置中 action 字段解析后的统一执行请求。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub action: String,
    pub cli: Option<AgentKind>,
    pub argument: Option<String>,
}

pub struct Resolver {
    catalog: Arc<IntentCatalog>,
}

impl Resolver {
    pub fn new(router: &RouterConfig, catalog: Arc<IntentCatalog>) -> anyhow::Result<Self> {
        let _ = router;
        Ok(Self { catalog })
    }

    pub fn decide(&self, intent: Intent) -> Decision {
        let action = self
            .catalog
            .find(&intent.name)
            .or_else(|| self.catalog.find("unknown"))
            .map(|spec| spec.action.clone())
            .unwrap_or_else(|| "system.unknown".to_owned());
        Decision {
            action,
            cli: intent.cli,
            argument: intent.argument,
        }
    }
}
