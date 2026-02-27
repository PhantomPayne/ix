use ix_core::error::{IxError, Result};
use ix_core::item::Item;
use ix_core::{Context, Provider};

use crate::{detect_git, open_repo};

#[derive(Default)]
pub struct GitLogProvider;

impl GitLogProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Provider for GitLogProvider {
    fn name(&self) -> &str {
        "gl"
    }

    fn detect(&self, ctx: &Context) -> bool {
        detect_git(ctx)
    }

    fn list(&self, ctx: &Context) -> Result<Vec<Item>> {
        let repo = open_repo(ctx)?;

        let mut revwalk = repo
            .revwalk()
            .map_err(|e| IxError::Provider(format!("git revwalk: {e}")))?;

        // Start from HEAD
        if revwalk.push_head().is_err() {
            // Empy repo or broken HEAD
            return Ok(Vec::new());
        }

        let mut items = Vec::new();
        // Limit to 500 commits for speed, typical for interactive pickers
        let max_commits = 500;

        for oid in revwalk.take(max_commits) {
            let oid = oid.map_err(|e| IxError::Provider(format!("git oid: {e}")))?;
            let commit = repo
                .find_commit(oid)
                .map_err(|e| IxError::Provider(format!("git commit: {e}")))?;

            let hash = commit.id().to_string();
            let short_hash = &hash[..7];

            let summary = commit.summary().unwrap_or("").to_string();
            let label = format!("{short_hash} {summary}");

            let item = Item::new(0, hash, label);
            items.push(item);
        }

        Ok(items)
    }

    fn preview_cmd(&self, item: &Item) -> Option<String> {
        Some(format!("git show --color=always {}", item.raw))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{make_initial_commit, make_repo};
    use ix_core::test_utils::ctx_path;

    #[test]
    fn test_git_log_returns_commits() {
        let (td, repo) = make_repo();
        make_initial_commit(&repo);

        let items = GitLogProvider::new().list(&ctx_path(td.path())).unwrap();
        assert_eq!(items.len(), 1);
        assert!(items[0].label.contains("init"));
    }

    #[test]
    fn test_git_log_raw_is_full_hash() {
        let (td, repo) = make_repo();
        make_initial_commit(&repo);

        let items = GitLogProvider::new().list(&ctx_path(td.path())).unwrap();
        let item = &items[0];

        assert_eq!(item.raw.len(), 40); // the full 40 character sha1
        assert!(item.label.starts_with(&item.raw[..7]));
    }
}
