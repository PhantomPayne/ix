use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use crate::error::{IxError, Result};
use crate::item::Item;
use crate::provider::Context;
use crate::selection::Selection;

/// How old an index must be before it is considered stale.
pub const STALE_THRESHOLD_SECS: u64 = 300; // 5 minutes

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Index {
    pub provider: String,
    /// Stored as seconds since UNIX epoch for JSON portability.
    pub captured_at_secs: u64,
    pub items: Vec<Item>,
}

impl Index {
    pub fn new(provider: impl Into<String>, items: Vec<Item>) -> Self {
        let captured_at_secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            provider: provider.into(),
            captured_at_secs,
            items,
        }
    }

    pub fn captured_at(&self) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(self.captured_at_secs)
    }

    pub fn is_stale(&self) -> bool {
        let age = SystemTime::now()
            .duration_since(self.captured_at())
            .unwrap_or(Duration::from_secs(u64::MAX));
        age.as_secs() > STALE_THRESHOLD_SECS
    }

    pub fn age_secs(&self) -> u64 {
        SystemTime::now()
            .duration_since(self.captured_at())
            .unwrap_or(Duration::from_secs(u64::MAX))
            .as_secs()
    }

    pub fn write(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn read(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Err(IxError::NoIndex);
        }
        let data = std::fs::read_to_string(path)?;
        let index: Self = serde_json::from_str(&data)?;
        Ok(index)
    }

    pub fn resolve(&self, sel: &Selection) -> Result<Vec<&Item>> {
        sel.resolve(self)
    }

    /// Returns the index path keyed to the current terminal session.
    ///
    /// Uses `$IX_SESSION` (set by `ix init <shell>` to `$$`, the shell PID) so each
    /// terminal session has its own isolated index at `/tmp/ix-<session>`.
    /// Falls back to the current process PID if `IX_SESSION` is not set.
    pub fn index_path(_ctx: &Context) -> PathBuf {
        Self::session_index_path()
    }

    /// Returns the session-keyed index path (used by both `index_path` and CLI helpers).
    pub fn session_index_path() -> PathBuf {
        let session = std::env::var("IX_SESSION")
            .unwrap_or_else(|_| std::process::id().to_string());
        std::env::temp_dir().join(format!("ix-{}", session))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::Item;
    use tempfile::tempdir;

    #[test]
    fn test_index_roundtrip() {
        let items = vec![
            Item::new(1, "/path/to/foo.rs", "foo.rs")
                .with_status("M")
                .with_group("unstaged"),
            Item::new(2, "/path/to/bar.rs", "bar.rs")
                .with_status("??")
                .with_group("untracked"),
        ];
        let index = Index::new("git-status", items.clone());

        let td = tempdir().unwrap();
        let path = td.path().join("ix-index");
        index.write(&path).unwrap();
        let loaded = Index::read(&path).unwrap();

        assert_eq!(index.provider, loaded.provider);
        assert_eq!(index.items.len(), loaded.items.len());
        for (orig, loaded) in index.items.iter().zip(loaded.items.iter()) {
            assert_eq!(orig.slot, loaded.slot);
            assert_eq!(orig.raw, loaded.raw);
            assert_eq!(orig.label, loaded.label);
            assert_eq!(orig.status, loaded.status);
            assert_eq!(orig.group, loaded.group);
        }
    }

    #[test]
    fn test_index_stale_after_threshold() {
        let index = Index {
            provider: "test".into(),
            // Set timestamp far in the past
            captured_at_secs: 0,
            items: vec![],
        };
        assert!(index.is_stale());
    }

    #[test]
    fn test_index_not_stale_when_fresh() {
        let index = Index::new("test", vec![]);
        assert!(!index.is_stale());
    }
}
