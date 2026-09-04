//! Shell 候选命令的本地安全策略。

/// 只允许模型候选或用户编辑后的单行只读命令。
pub fn validate(command: &str) -> Result<(), String> {
    let command = command.trim();
    if command.is_empty() {
        return Err("该请求需要高风险操作，未生成可执行命令。".to_owned());
    }
    if command.contains(['\n', '\r', '\0']) {
        return Err("只支持单行 Shell 命令。".to_owned());
    }
    if command.contains([';', '|', '&', '>', '<', '`']) || command.contains("$(") {
        return Err("候选命令包含组合、重定向或命令替换，已拒绝生成。".to_owned());
    }
    const ALLOWED: &[&str] = &[
        "ls", "pwd", "find", "rg", "grep", "cat", "head", "tail", "wc", "du", "df", "ps", "git",
        "which", "echo", "tree", "stat",
    ];
    let executable = command.split_whitespace().next().unwrap_or_default();
    if !ALLOWED.contains(&executable) {
        return Err("候选命令不在只读命令白名单中。请使用 `$ 命令` 明确手动执行。".to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate;

    #[test]
    fn rejects_unsafe_commands() {
        assert!(validate("ls -la").is_ok());
        assert!(validate("rm -rf demo").is_err());
        assert!(validate("ls | grep demo").is_err());
    }
}
