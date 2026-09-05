// ！用户输入规范化:不做意图判定

/// 规范化后的本轮输入
#[derive(Debug, Clone)]
pub(crate) struct NormalizedInput {
    /// trim 后的原文（中文匹配用）
    pub text: String,
    /// 小写副本（ASCII 关键词匹配用）
    pub lower: String,
}

/// trim + 小写副本；不做意图判定。
pub fn prepare(input: &str) -> NormalizedInput {
    let text = input.trim().to_string();
    let lower = text.to_lowercase();
    NormalizedInput { text, lower }
}
