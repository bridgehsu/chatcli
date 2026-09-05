use std::{
    fs,
    path::{Path, PathBuf},
};

/// 文件系统检索返回的事实数据；调用方决定是否可用于创建终端。
#[derive(Debug)]
pub enum SearchResult {
    Unique(PathBuf),
    Multiple(Vec<PathBuf>),
    NotFound,
}

/// Agent 的本地文件系统工具：解析工作目录、列目录和搜索文件。
#[derive(Clone)]
pub struct WorkspaceSearchTool {
    home: PathBuf,
    roots: Vec<PathBuf>,
}

impl WorkspaceSearchTool {
    pub fn new(home: PathBuf) -> Self {
        Self {
            roots: vec![home.clone()],
            home,
        }
    }

    pub async fn resolve_workspace(&self, input: &str) -> Result<SearchResult, String> {
        let input = input.trim();
        if input.is_empty() {
            return Err("目录不能为空。".into());
        }
        if looks_like_path(input) {
            return validate_path(&self.home, input).map(SearchResult::Unique);
        }
        let query = workspace_query(input);
        let roots = self.roots.clone();
        tokio::task::spawn_blocking(move || search_directories(&roots, &query, 5))
            .await
            .map_err(|error| format!("目录搜索任务失败：{error}"))?
    }

    /// 列出 Home 目录下可作为工作区起点的可见目录。
    pub async fn list_directories(&self) -> Result<Vec<PathBuf>, String> {
        let home = self.home.clone();
        tokio::task::spawn_blocking(move || list_directories(&home))
            .await
            .map_err(|error| format!("目录列表任务失败：{error}"))?
    }
}

fn list_directories(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut directories = fs::read_dir(root)
        .map_err(|error| format!("无法读取目录 {}：{error}", root.display()))?
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let name = entry.file_name();
            (!name.to_string_lossy().starts_with('.') && path.is_dir()).then_some(path)
        })
        .collect::<Vec<_>>();
    directories.sort();
    directories.truncate(20);
    Ok(directories)
}

fn validate_path(home: &Path, input: &str) -> Result<PathBuf, String> {
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
    if path.is_dir() {
        Ok(path)
    } else {
        Err(format!("这不是目录：{}", path.display()))
    }
}

fn looks_like_path(input: &str) -> bool {
    input == "~" || input.starts_with("~/") || input.starts_with('/') || input.starts_with('.')
}

fn workspace_query(input: &str) -> String {
    input
        .trim()
        .trim_start_matches("打开")
        .trim_start_matches("进入")
        .trim_start_matches("使用")
        .trim_end_matches("这个目录")
        .trim_end_matches("目录")
        .trim()
        .to_owned()
}
fn search_directories(
    roots: &[PathBuf],
    query: &str,
    max_depth: usize,
) -> Result<SearchResult, String> {
    Ok(to_result(search(roots, query, max_depth, true)))
}
fn to_result(mut matches: Vec<PathBuf>) -> SearchResult {
    matches.sort();
    matches.dedup();
    match matches.len() {
        0 => SearchResult::NotFound,
        1 => SearchResult::Unique(matches.remove(0)),
        _ => SearchResult::Multiple(matches),
    }
}
fn search(roots: &[PathBuf], query: &str, max_depth: usize, directories: bool) -> Vec<PathBuf> {
    let mut matches = Vec::new();
    for root in roots {
        walk(root, query, max_depth, directories, &mut matches);
    }
    matches
}
fn walk(path: &Path, query: &str, remaining: usize, directories: bool, matches: &mut Vec<PathBuf>) {
    if remaining == 0 || matches.len() >= 20 {
        return;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        if matches.len() >= 20 {
            return;
        }
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || matches!(name.as_str(), "node_modules" | "target" | "Library") {
            continue;
        }
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let matches_query = name.eq_ignore_ascii_case(query);
        if file_type.is_dir() {
            if directories && matches_query {
                matches.push(path.clone());
            }
            walk(&path, query, remaining - 1, directories, matches);
        } else if !directories && matches_query {
            matches.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extracts_workspace_name_from_a_natural_request() {
        assert_eq!(workspace_query("打开 chatcli 这个目录"), "chatcli");
    }
}
