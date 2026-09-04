//! 本地持久化适配层。
//!
//! 后续可把 JSON 文件实现拆为 session_store、terminal_store 和 context_store，
//! 并在不改变领域模型的情况下替换为 SQLite 或远程存储。

pub mod context_store;
pub mod session_store;
pub mod terminal_store;
