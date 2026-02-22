use ix_core::{Context, Item, Provider, ProviderOption};
use ix_core::error::{IxError, Result};
use git2::Repository;

pub struct GitStashProvider;

impl GitStashProvider {
    pub fn new() -> Self { Self }
}

impl Default for GitStashProvider {
    fn default() -> Self { Self::new() }
}

impl Provider for GitStashProvider {
    fn name(&self) -> &str {
        "git-stash"
    }

    fn detect(ctx: &Context) -> bool {
        Repository::discover(&ctx.cwd).is_ok()
    }

    fn options() -> Vec<ProviderOption> {
        vec![]
    }

    fn list(&self, ctx: &Context) -> Result<Vec<Item>> {
        let mut repo = Repository::discover(&ctx.cwd)
            .map_err(|e| IxError::Provider(format!("git: {e}")))?;

        let mut items = Vec::new();
        let mut slot = 1usize;

        repo.stash_foreach(|index, message, _oid| {
            let raw = format!("stash@{{{index}}}");
            let label = format!("[{index}] {message}");
            let item = Item::new(slot, &raw, &label);
            items.push(item);
            slot += 1;
            true // continue iterating
        })
        .map_err(|e| IxError::Provider(format!("stash foreach: {e}")))?;

        Ok(items)
    }

    fn preview_cmd(&self, item: &Item) -> Option<String> {
        Some(format!("git stash show -p {}", shell_quote(&item.raw)))
    }
}

fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::tempdir;

    fn make_repo() -> (tempfile::TempDir, git2::Repository) {
        let td = tempdir().unwrap();
        let repo = git2::Repository::init(td.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "test").unwrap();
        config.set_str("user.email", "test@test.com").unwrap();
        (td, repo)
    }

    fn make_initial_commit(repo: &git2::Repository) {
        let sig = repo.signature().unwrap();
        let tree_id = repo.index().unwrap().write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[]).unwrap();
    }

    fn ctx(path: &Path) -> Context {
        Context::new(path.to_path_buf())
    }

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
