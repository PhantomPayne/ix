use crate::{detect_git, open_repo};
use ix_core::error::{IxError, Result};
use ix_core::{Context, Item, Provider};
use shell_escape::escape;
use std::borrow::Cow;

#[derive(Default)]
pub struct GitStashProvider;

impl GitStashProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Provider for GitStashProvider {
    fn name(&self) -> &str {
        "git-stash"
    }

    fn detect(&self, ctx: &Context) -> bool {
        detect_git(ctx)
    }

    fn list(&self, ctx: &Context) -> Result<Vec<Item>> {
        let mut repo = open_repo(ctx)?;

        let mut items = Vec::new();

        repo.stash_foreach(|index, message, _oid| {
            let raw = format!("stash@{{{index}}}");
            let label = format!("[{index}] {message}");
            let item = Item::new(0, &raw, &label);
            items.push(item);
            true // continue iterating
        })
        .map_err(|e| IxError::Provider(format!("stash foreach: {e}")))?;

        Ok(items)
    }

    fn preview_cmd(&self, item: &Item) -> Option<String> {
        Some(format!(
            "git stash show -p {}",
            escape(Cow::Borrowed(&item.raw))
        ))
    }
}

#[cfg(test)]
use ix_core::test_utils::ctx_path as ctx;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{make_initial_commit, make_repo};

    #[test]
    fn test_stash_lists_stashes() {
        let (td, mut repo) = make_repo();
        // Stage and commit foo.rs so stash has a tracked modified file to save
        std::fs::write(td.path().join("foo.rs"), "original").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("foo.rs")).unwrap();
        index.write().unwrap();
        make_initial_commit(&repo);
        // Modify the tracked file (unstaged) so stash has something to save
        std::fs::write(td.path().join("foo.rs"), "changes").unwrap();

        let sig = repo.signature().unwrap();
        repo.stash_save(&sig, "my stash message", None).unwrap();

        let items = GitStashProvider::new().list(&ctx(td.path())).unwrap();

        assert_eq!(items.len(), 1);
        assert!(items[0].label.contains("my stash message"));
        assert_eq!(items[0].raw, "stash@{0}");
    }

    #[test]
    fn test_stash_empty() {
        let (td, _repo) = make_repo();
        let items = GitStashProvider::new().list(&ctx(td.path())).unwrap();
        assert!(items.is_empty());
    }
}
