//! 意图目录配置：意图名称、说明、示例及要执行的动作均在 YAML 中维护。

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::{collections::HashSet, path::Path};

#[derive(Debug, Clone, Default, Deserialize)]
pub struct IntentCatalog {
    pub intents: Vec<IntentSpec>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IntentSpec {
    pub name: String,
    pub action: String,
    pub description: String,
    #[serde(default)]
    pub examples: Vec<String>,
}

impl IntentCatalog {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read intent catalog: {}", path.display()))?;
        let catalog: Self = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse intent catalog: {}", path.display()))?;
        catalog.validate()?;
        Ok(catalog)
    }

    pub fn find(&self, name: &str) -> Option<&IntentSpec> {
        self.intents.iter().find(|spec| spec.name == name)
    }

    /// 为当前识别 Profile 生成受限候选集，避免模型返回状态下不允许的意图。
    pub fn system_prompt_for(&self, allowed: &[&str]) -> String {
        let intents = self
            .intents
            .iter()
            .filter(|spec| allowed.is_empty() || allowed.contains(&spec.name.as_str()))
            .map(|spec| {
                let examples = if spec.examples.is_empty() {
                    "无".to_owned()
                } else {
                    spec.examples.join("、")
                };
                format!("- {}：{}。示例：{}", spec.name, spec.description, examples)
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!("你是 ChatCLI 的意图分类器，用户主要使用中文。只返回 JSON 对象，不要解释、Markdown 或 shell 命令。\n可选意图：\n{}\n输出格式：{{\"intent\":\"意图名称\",\"cli\":\"codex|cursor|null\",\"confidence\":0.0}}。只有 choose_cli 时 cli 才填写 codex 或 cursor；无法可靠判断时使用 unknown。", intents)
    }

    fn validate(&self) -> Result<()> {
        let mut names = HashSet::new();
        for spec in &self.intents {
            if spec.name.trim().is_empty()
                || spec.action.trim().is_empty()
                || spec.description.trim().is_empty()
            {
                bail!("Intent name, action and description cannot be empty");
            }
            if !names.insert(spec.name.as_str()) {
                bail!("Duplicate intent name in catalog: {}", spec.name);
            }
        }
        if !names.contains("unknown") {
            bail!("Intent catalog is missing required fallback intent: unknown");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn loads_catalog_from_yaml() {
        let catalog = IntentCatalog::load("config/intents.yaml").unwrap();
        assert_eq!(catalog.find("choose_cli").unwrap().action, "cli.choose");
    }
}
