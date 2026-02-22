use ix_core::{Context, Item, Provider, ProviderOption};
use ix_core::error::{IxError, Result};
use git2::{BranchType, Repository};

pub struct GitBranchProvider;

impl GitBranchProvider {
    pub fn new() -> Self { Self }
}

impl Default for GitBranchProvider {
    fn default() -> Self { Self::new() }
}

impl Provider for GitBranchProvider {
    fn name(&self) -> &str {
        "git-branches"
    }

    fn detect(ctx: &Context) -> bool {
        Repository::discover(&ctx.cwd).is_ok()
    }

    fn options() -> Vec<ProviderOption> {
        vec![]
    }

    fn list(&self, ctx: &Context) -> Result<Vec<Item>> {
        let repo = Repository::discover(&ctx.cwd)
            .map_err(|e| IxError::Provider(format!("git: {e}")))?;

        let head_name = repo.head().ok()
            .and_then(|h| h.shorthand().map(|s| s.to_string()));

        let mut items = Vec::new();
        let mut slot = 1usize;

        // Local branches
        let local = repo.branches(Some(BranchType::Local))
            .map_err(|e| IxError::Provider(format!("git branches: {e}")))?;
        for branch_result in local {
            let (branch, _) = branch_result
                .map_err(|e| IxError::Provider(format!("branch iter: {e}")))?;
            let name = branch.name()
                .map_err(|e| IxError::Provider(format!("branch name: {e}")))?
                .unwrap_or("")
                .to_string();
            let is_head = head_name.as_deref() == Some(&name);
            let mut item = Item::new(slot, &name, &name)
                .with_group("local");
            if is_head {
                item = item.with_status("*");
            }
            items.push(item);
            slot += 1;
        }

        // Remote branches
        let remote = repo.branches(Some(BranchType::Remote))
            .map_err(|e| IxError::Provider(format!("git remote branches: {e}")))?;
        for branch_result in remote {
            let (branch, _) = branch_result
                .map_err(|e| IxError::Provider(format!("branch iter: {e}")))?;
            let name = branch.name()
                .map_err(|e| IxError::Provider(format!("branch name: {e}")))?
                .unwrap_or("")
                .to_string();
            let item = Item::new(slot, &name, &name)
                .with_group("remote");
            items.push(item);
            slot += 1;
        }

        Ok(items)
    }

    fn preview_cmd(&self, item: &Item) -> Option<String> {
        Some(format!("git log --oneline -15 {}", shell_quote(&item.raw)))
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
    fn test_branches_lists_local() {
        let (td, repo) = make_repo();
        make_initial_commit(&repo);
        repo.branch(
            "feature-x",
            &repo.head().unwrap().peel_to_commit().unwrap(),
            false,
        ).unwrap();

        let items = GitBranchProvider::new().list(&ctx(td.path())).unwrap();

        assert!(items.iter().any(|i| i.label == "feature-x"));
        assert!(items.iter().all(|i| i.group.as_deref() == Some("local")));
    }

    #[test]
    fn test_branches_raw_is_branch_name() {
        let (td, repo) = make_repo();
        make_initial_commit(&repo);

        let items = GitBranchProvider::new().list(&ctx(td.path())).unwrap();

        // raw should be just the branch name for use in git commands
        assert!(items.iter().any(|i| i.raw == "main" || i.raw == "master"));
    }
}
