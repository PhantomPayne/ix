use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{IxError, Result};
use crate::item::Item;
use crate::selection::Selection;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Index {
    pub provider: String,
    pub items: Vec<Item>,
}

impl Index {
    pub fn new(provider: impl Into<String>, items: Vec<Item>) -> Self {
        Self {
            provider: provider.into(),
            items,
        }
    }

    pub fn write(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // Rotate: copy current index → prev before overwriting
        if path.exists() {
            let prev = path.with_extension("prev.json");
            let _ = std::fs::copy(path, &prev);
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

    /// Returns the digit-width of the largest slot number, used to pad
    /// slot columns consistently across display and the TUI picker.
    pub fn max_slot_width(&self) -> usize {
        self.items
            .iter()
            .map(|i| i.slot)
            .max()
            .unwrap_or(0)
            .to_string()
            .len()
    }

    /// Build a `slot → &Item` map for O(1) lookup during selection resolution.
    pub fn slot_map(&self) -> HashMap<usize, &Item> {
        self.items.iter().map(|i| (i.slot, i)).collect()
    }

    /// Returns the index path for a terminal session.
    ///
    /// Path: `~/.cache/ix/sessions/<session_id>/index.json`
    pub fn index_path(session_id: &str) -> PathBuf {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("ix")
            .join("sessions")
            .join(session_id);
        cache_dir.join("index.json")
    }

    /// Returns the previous index path for a terminal session.
    ///
    /// Path: `~/.cache/ix/sessions/<session_id>/index.prev.json`
    pub fn prev_path(session_id: &str) -> PathBuf {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("ix")
            .join("sessions")
            .join(session_id);
        cache_dir.join("index.prev.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::{Category, Item};
    use tempfile::tempdir;

    #[test]
    fn test_index_roundtrip() {
        let items = vec![
            Item::new(1, "/path/to/foo.rs", "foo.rs")
                .with_status("M", Category::Warning)
                .with_group("unstaged"),
            Item::new(2, "/path/to/bar.rs", "bar.rs")
                .with_status("??", Category::Neutral)
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
    fn test_index_path_uses_session_id() {
        let path = Index::index_path("12345");
        assert!(path.ends_with("ix/sessions/12345/index.json"));
    }
}
