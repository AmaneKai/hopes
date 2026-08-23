use chrono::{DateTime, Local};
use std::collections::VecDeque;

/// Caps memory use; the trail is in-memory/session-scoped, not persisted.
const MAX_ENTRIES: usize = 50;

#[derive(Clone, Debug)]
pub struct AuditEvent {
    pub timestamp: DateTime<Local>,
    pub message: String,
}

#[derive(Default)]
pub struct AuditLog {
    entries: VecDeque<AuditEvent>,
}

impl AuditLog {
    pub fn push(&mut self, message: impl Into<String>) {
        if self.entries.len() >= MAX_ENTRIES {
            self.entries.pop_back();
        }
        self.entries.push_front(AuditEvent {
            timestamp: Local::now(),
            message: message.into(),
        });
    }

    /// Most recent first.
    pub fn recent(&self, n: usize) -> impl Iterator<Item = &AuditEvent> {
        self.entries.iter().take(n)
    }
}
