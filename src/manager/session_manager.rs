//! Agent Session 的模型、持久化与上下文记录。

use crate::agent::{agent::PendingLaunch, AgentKind};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// 一个 Agent 会话在 Telegram 流程中的阶段。
///
/// 这描述的是“用户正进行到哪一步”，而不是 tmux 进程自身的运行状态。
pub enum SessionState {
    /// 刚创建会话，尚未选择 CodeX 或 Cursor。
    Initial,
    /// 已选择 CLI，下一步通常是选择工作目录。
    CliSelected { kind: AgentKind },
    /// 正等待用户从候选目录中选择工作目录。
    WaitingWorkspace {
        /// 可能为空：例如用户先要求列出目录、尚未选择 CLI。
        kind: Option<AgentKind>,
        /// 本次等待用户选择的目录候选项。
        candidates: Vec<PathBuf>,
        /// 用户已经解析出的工作目录；保留字段便于恢复中断流程。
        #[serde(default)]
        workspace: Option<PathBuf>,
    },
    /// 已关联到一个正在使用的 tmux CLI 会话。
    ActiveTerminal {
        kind: AgentKind,
        terminal_session_id: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// Agent 会话是否仍可被用户继续使用。
pub enum SessionStatus {
    /// 正常可用，也可能尚未打开 CLI。
    Active,
    /// 已关闭；保留记录但不能重新切换到该会话。
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// 一个可持久化的 Agent 会话。
///
/// 一个 Telegram chat 可以拥有多个 Session，但 `current` 映射始终只指向其中一个。
pub struct Session {
    /// 全局唯一的 Agent 会话 ID，例如 `agent-<chat_id>-<timestamp>`。
    pub id: String,
    /// Telegram chat ID，用于隔离不同用户的会话。
    pub chat_id: i64,
    /// 当前 Telegram 流程状态。
    pub state: SessionState,
    /// 会话是否已经关闭。
    pub status: SessionStatus,
    /// 创建时间，Unix 毫秒。
    pub created: u64,
    /// 最近一次状态更新的时间，Unix 毫秒。
    pub updated: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// 某个 Agent 会话中的一条上下文事件。
///
/// 事件以 JSON Lines 追加到历史文件，后续可用于恢复上下文或提供给模型。
pub struct ContextEvent {
    pub session_id: String,
    pub chat_id: i64,
    pub role: String,
    pub content: String,
    pub created_at: u64,
}

#[derive(Default, Serialize, Deserialize)]
/// 写入 `agent-sessions.json` 的完整内容。
///
/// `sessions` 保存历史会话；`current` 保存每个 chat 当前激活的会话 ID。
struct StoredAgentSessions {
    sessions: Vec<Session>,
    current: HashMap<i64, String>,
}

/// 管理 Agent Session 的创建、恢复、切换、状态保存及上下文追加。
pub struct SessionManager {
    /// Agent 会话状态快照文件：`~/.chatcli/agent-sessions.json`。
    state_file: PathBuf,
    /// 上下文事件追加文件：`~/.chatcli/agent-messages.jsonl`。
    history_file: PathBuf,
    /// 进程内缓存；Mutex 保证多个 Telegram 更新并发时状态一致。
    sessions: Mutex<StoredAgentSessions>,
}

impl SessionManager {
    /// 从本地磁盘读取已有会话；文件不存在或解析失败时从空状态开始。
    pub fn new(home: &PathBuf) -> Self {
        let state_file = home.join(".chatcli").join("agent-sessions.json");
        let history_file = home.join(".chatcli").join("agent-messages.jsonl");
        let sessions = fs::read_to_string(&state_file)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        Self {
            state_file,
            history_file,
            sessions: Mutex::new(sessions),
        }
    }

    /// 返回每个 chat 当前仍处于 Active 状态的会话，供应用启动时恢复内存状态。
    pub async fn restore(&self) -> HashMap<i64, Session> {
        let stored = self.sessions.lock().await;
        stored
            .current
            .iter()
            .filter_map(|(chat, id)| {
                stored
                    .sessions
                    .iter()
                    .find(|s| s.id == *id && s.status == SessionStatus::Active)
                    .cloned()
                    .map(|s| (*chat, s))
            })
            .collect()
    }

    /// 获取某个 chat 的全部会话（按最近更新时间倒序）和当前会话 ID。
    pub async fn list(&self, chat_id: i64) -> (Vec<Session>, Option<String>) {
        let stored = self.sessions.lock().await;
        let mut sessions = stored
            .sessions
            .iter()
            .filter(|s| s.chat_id == chat_id)
            .cloned()
            .collect::<Vec<_>>();
        sessions.sort_by_key(|s| std::cmp::Reverse(s.updated));
        (sessions, stored.current.get(&chat_id).cloned())
    }

    /// 获取某个 chat 当前选中的有效会话。
    pub async fn current(&self, chat_id: i64) -> Option<Session> {
        let stored = self.sessions.lock().await;
        let id = stored.current.get(&chat_id)?;
        stored
            .sessions
            .iter()
            .find(|s| s.id == *id && s.status == SessionStatus::Active)
            .cloned()
    }

    /// 切换当前 Agent 会话。
    ///
    /// 只有属于该 chat 且处于 Active 的会话才允许切换，避免跨用户访问会话。
    pub async fn select(&self, chat_id: i64, id: &str) -> Option<Session> {
        let mut stored = self.sessions.lock().await;
        let session = stored
            .sessions
            .iter()
            .find(|s| s.id == id && s.chat_id == chat_id && s.status == SessionStatus::Active)
            .cloned()?;
        stored.current.insert(chat_id, id.to_owned());
        self.save(&stored);
        Some(session)
    }

    /// 为当前会话记录一条上下文。
    ///
    /// 若 chat 还没有会话，会先创建一个 Initial 会话，再把事件追加到 JSONL 文件。
    pub async fn record(&self, chat_id: i64, role: &str, content: impl Into<String>) {
        let session_id = {
            let mut stored = self.sessions.lock().await;
            let id = current_or_create(&mut stored, chat_id);
            self.save(&stored);
            id
        };
        let event = ContextEvent {
            session_id,
            chat_id,
            role: role.to_owned(),
            content: content.into(),
            created_at: now_millis(),
        };
        if let Some(parent) = self.history_file.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(line) = serde_json::to_string(&event) {
            use std::io::Write;
            if let Ok(mut file) = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.history_file)
            {
                let _ = writeln!(file, "{line}");
            }
        }
    }

    /// 创建新的 Initial 会话并设为当前会话。
    ///
    /// 此方法不会关闭旧会话，也不会终止其关联的 tmux CLI；旧会话仍可从列表切回。
    pub async fn new_session(&self, chat_id: i64) {
        let mut stored = self.sessions.lock().await;
        let now = now_millis();
        let id = format!("agent-{chat_id}-{now}");
        stored.sessions.push(Session {
            id: id.clone(),
            chat_id,
            state: SessionState::Initial,
            status: SessionStatus::Active,
            created: now,
            updated: now,
        });
        stored.current.insert(chat_id, id);
        self.save(&stored);
    }

    /// 根据 Agent 的运行时信息生成并保存当前会话状态。
    ///
    /// 优先级为：等待目录 > 活动终端 > 已选择 CLI > 初始状态。
    pub async fn save_state(
        &self,
        chat_id: i64,
        selected: Option<AgentKind>,
        pending: Option<PendingLaunch>,
        terminal: Option<(AgentKind, String)>,
    ) {
        let state = match pending {
            Some(p) => SessionState::WaitingWorkspace {
                kind: p.kind,
                candidates: p.candidates,
                workspace: p.workspace,
            },
            None => match terminal {
                Some((kind, id)) => SessionState::ActiveTerminal {
                    kind,
                    terminal_session_id: id,
                },
                None => selected
                    .map(|kind| SessionState::CliSelected { kind })
                    .unwrap_or(SessionState::Initial),
            },
        };
        let mut stored = self.sessions.lock().await;
        let id = current_or_create(&mut stored, chat_id);
        if let Some(session) = stored.sessions.iter_mut().find(|s| s.id == id) {
            session.state = state;
            session.updated = now_millis();
        }
        self.save(&stored);
    }

    /// 将完整会话快照同步写入本地 JSON 文件。
    fn save(&self, stored: &StoredAgentSessions) {
        if let Some(parent) = self.state_file.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_vec_pretty(stored) {
            let _ = fs::write(&self.state_file, json);
        }
    }
}

/// 返回 chat 当前的会话 ID；如果还不存在则创建一个 Initial 会话。
fn current_or_create(stored: &mut StoredAgentSessions, chat_id: i64) -> String {
    if let Some(id) = stored.current.get(&chat_id) {
        return id.clone();
    }
    let now = now_millis();
    let id = format!("agent-{chat_id}-{now}");
    stored.sessions.push(Session {
        id: id.clone(),
        chat_id,
        state: SessionState::Initial,
        status: SessionStatus::Active,
        created: now,
        updated: now,
    });
    stored.current.insert(chat_id, id.clone());
    id
}

/// 统一生成持久化字段使用的 Unix 毫秒时间戳。
fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
