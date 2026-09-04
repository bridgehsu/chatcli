//! Agent 对话上下文的 JSON Lines 持久化。

use crate::domain::ContextEvent;
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

pub struct ContextStore {
    history_file: PathBuf,
}

impl ContextStore {
    pub fn new(home: &PathBuf) -> Self {
        Self {
            history_file: home.join(".chatcli").join("agent-messages.jsonl"),
        }
    }

    pub fn record(&self, session_id: String, chat_id: i64, role: &str, content: impl Into<String>) {
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

    pub fn recent(&self, session_id: &str, chat_id: i64, limit: usize) -> Vec<ContextEvent> {
        let Ok(history) = fs::read_to_string(&self.history_file) else {
            return vec![];
        };
        let mut events = history
            .lines()
            .filter_map(|line| serde_json::from_str::<ContextEvent>(line).ok())
            .filter(|event| {
                event.chat_id == chat_id
                    && event.session_id == session_id
                    && matches!(event.role.as_str(), "user" | "agent")
            })
            .collect::<Vec<_>>();
        if events.len() > limit {
            events.drain(..events.len() - limit);
        }
        for event in &mut events {
            event.content = event.content.chars().take(1_200).collect();
        }
        events
    }

    pub fn clear_chat(&self, chat_id: i64) {
        let Ok(history) = fs::read_to_string(&self.history_file) else {
            return;
        };
        let retained = history
            .lines()
            .filter(|line| match serde_json::from_str::<ContextEvent>(line) {
                Ok(event) => event.chat_id != chat_id,
                Err(_) => true,
            })
            .collect::<Vec<_>>();
        let content = if retained.is_empty() {
            String::new()
        } else {
            format!("{}\n", retained.join("\n"))
        };
        let _ = fs::write(&self.history_file, content);
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
