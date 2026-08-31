use std::path::{Path, PathBuf};

/// Resolves a user-selected working directory without depending on a Git repo.
pub fn resolve(home: &Path, input: &str) -> Result<PathBuf, String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("目录不能为空。".into());
    }
    let path = match input {
        "~" => home.to_path_buf(),
        _ if input.starts_with("~/") => home.join(&input[2..]),
        _ => {
            let path = PathBuf::from(input);
            if path.is_absolute() {
                path
            } else {
                home.join(path)
            }
        }
    };
    let path = path
        .canonicalize()
        .map_err(|_| format!("目录不存在或不可访问：{}", path.display()))?;
    if !path.is_dir() {
        return Err(format!("这不是目录：{}", path.display()));
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_home_shortcut() {
        let home = std::env::temp_dir();
        assert_eq!(resolve(&home, "~").unwrap(), home.canonicalize().unwrap());
    }
}
