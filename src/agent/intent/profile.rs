//! 将当前 Session 快照转换为本轮允许识别的意图范围与兜底策略。

use crate::{
    agent::TerminalKind,
    app::agent_runtime::{CurrentSession, TerminalState},
    domain::AgentSessionState,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackPolicy {
    AgentChat,
    WorkspaceInput,
    TerminalInput,
    ShellRequest,
    WaitingShellConfirmation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileKind {
    Initial,
    WaitingWorkspace,
    Shell,
    Codex,
    Cursor,
    WaitingShellConfirmation,
}

#[derive(Debug, Clone)]
pub struct RecognitionProfile {
    pub kind: ProfileKind,
    pub allowed: &'static [&'static str],
    pub fallback: FallbackPolicy,
}

impl RecognitionProfile {
    pub fn from_current(current: &CurrentSession) -> Self {
        if matches!(
            &current.state,
            Some(AgentSessionState::WaitingWorkspace { .. })
        ) {
            return Self::waiting_workspace();
        }
        if matches!(
            &current.state,
            Some(AgentSessionState::WaitingShellCommand { .. })
        ) {
            return Self::waiting_shell_confirmation();
        }
        match &current.terminal {
            TerminalState::Active { kind, .. } => Self::terminal(*kind),
            TerminalState::None | TerminalState::Selected(_) => Self::initial(),
        }
    }

    pub fn allows(&self, intent: &str) -> bool {
        self.allowed.contains(&intent)
    }

    fn initial() -> Self {
        Self {
            kind: ProfileKind::Initial,
            allowed: &[
                "open_terminal",
                "choose_cli",
                "new_session",
                "close_session",
                "list_session",
                "switch_session",
                "switch_agent_session",
                "self_introduction",
                "help",
                "reset",
            ],
            fallback: FallbackPolicy::AgentChat,
        }
    }

    fn waiting_workspace() -> Self {
        Self {
            kind: ProfileKind::WaitingWorkspace,
            allowed: &["list_directories", "cancel", "help", "reset"],
            fallback: FallbackPolicy::WorkspaceInput,
        }
    }

    fn terminal(kind: TerminalKind) -> Self {
        let (kind, allowed) = match kind {
            TerminalKind::Shell => (ProfileKind::Shell, TERMINAL_ALLOWED),
            TerminalKind::Codex => (ProfileKind::Codex, TERMINAL_ALLOWED),
            TerminalKind::Cursor => (ProfileKind::Cursor, TERMINAL_ALLOWED),
        };
        Self {
            kind,
            allowed,
            fallback: if kind == ProfileKind::Shell {
                FallbackPolicy::ShellRequest
            } else {
                FallbackPolicy::TerminalInput
            },
        }
    }

    fn waiting_shell_confirmation() -> Self {
        Self {
            kind: ProfileKind::WaitingShellConfirmation,
            allowed: &["help", "reset", "shell_edit"],
            fallback: FallbackPolicy::WaitingShellConfirmation,
        }
    }
}

const TERMINAL_ALLOWED: &[&str] = &[
    "open_terminal",
    "choose_cli",
    "close_terminal",
    "list_terminal",
    "switch_terminal",
    "switch_terminal_id",
    "new_session",
    "close_session",
    "list_session",
    "switch_session",
    "switch_agent_session",
    "self_introduction",
    "help",
    "reset",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waiting_workspace_is_a_strict_profile() {
        let profile = RecognitionProfile::from_current(&CurrentSession {
            id: Some("agent-1".to_owned()),
            state: Some(AgentSessionState::WaitingWorkspace {
                kind: None,
                candidates: vec![],
                workspace: None,
            }),
            terminal: TerminalState::None,
        });
        assert_eq!(profile.kind, ProfileKind::WaitingWorkspace);
        assert_eq!(profile.fallback, FallbackPolicy::WorkspaceInput);
        assert!(profile.allows("cancel"));
        assert!(!profile.allows("open_terminal"));
    }

    #[test]
    fn codex_profile_defaults_to_terminal_input() {
        let profile = RecognitionProfile::from_current(&CurrentSession {
            id: Some("agent-1".to_owned()),
            state: None,
            terminal: TerminalState::Active {
                kind: TerminalKind::Codex,
                terminal_id: "codex-1".to_owned(),
            },
        });
        assert_eq!(profile.kind, ProfileKind::Codex);
        assert_eq!(profile.fallback, FallbackPolicy::TerminalInput);
        assert!(profile.allows("switch_terminal"));
    }
}
