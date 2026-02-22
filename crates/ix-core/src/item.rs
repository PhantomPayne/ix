use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub slot: usize,
    /// What gets shell-expanded (filepath, pid, branch name, etc.)
    pub raw: String,
    /// Display name (may differ from raw)
    pub label: String,
    /// "M", "A", "D", "??", "running", "stopped", etc.
    pub status: Option<String>,
    /// Display-only grouping: "staged", "unstaged", "local", "remote", etc.
    pub group: Option<String>,
    /// Provider-specific extras
    pub meta: HashMap<String, serde_json::Value>,
}

impl Item {
    pub fn new(slot: usize, raw: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            slot,
            raw: raw.into(),
            label: label.into(),
            status: None,
            group: None,
            meta: HashMap::new(),
        }
    }

    pub fn with_status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
    }

    pub fn with_group(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
    }

    pub fn with_meta(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.meta.insert(key.into(), value);
        self
    }
}
