use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Positive, // e.g., added, running
    Negative, // e.g., deleted, zombie
    Warning,  // e.g., modified, paused
    Neutral,  // e.g., untracked, exited
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemStatus {
    pub text: String,
    pub category: Category,
}

impl ItemStatus {
    /// Convenience: return the status text as `&str` for assertions and display.
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub slot: usize,
    /// What gets shell-expanded (filepath, pid, branch name, etc.)
    pub raw: String,
    /// Display name (may differ from raw)
    pub label: String,
    /// "M", "A", "D", "??", "running", "stopped", etc.
    pub status: Option<ItemStatus>,
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

    pub fn with_status(mut self, text: impl Into<String>, category: Category) -> Self {
        self.status = Some(ItemStatus {
            text: text.into(),
            category,
        });
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

/// Assign sequential slot numbers (starting from 1) to a list of items.
///
/// Providers can build items without worrying about slot numbering, then call
/// this at the end to stamp each item with its final slot.
pub fn assign_slots(mut items: Vec<Item>) -> Vec<Item> {
    for (i, item) in items.iter_mut().enumerate() {
        item.slot = i + 1;
    }
    items
}

/// Reorder items into visual group order: groups appear in first-seen insertion
/// order, and items within each group keep their relative order.
///
/// This ensures that when `assign_slots` runs afterwards, slot numbers
/// match the visual display order (no gaps or jumps within groups).
pub fn reorder_by_group(items: Vec<Item>) -> Vec<Item> {
    use indexmap::IndexSet;
    let groups: IndexSet<Option<String>> = items.iter().map(|i| i.group.clone()).collect();
    let mut result = Vec::with_capacity(items.len());
    for group in groups {
        result.extend(items.iter().filter(|i| i.group == group).cloned());
    }
    result
}
