use crate::{detect_git, open_repo};
use git2::BranchType;
use ix_core::error::{IxError, Result};
use ix_core::item::{Category, Item};
use ix_core::{Context, Provider};
use shell_escape::escape;
use std::borrow::Cow;

#[derive(Default)]
pub struct GitBranchProvider;

impl GitBranchProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Provider for GitBranchProvider {
    fn name(&self) -> &str {
        "git-branches"
    }

    fn detect(&self, ctx: &Context) -> bool {
        detect_git(ctx)
    }

    fn list(&self, ctx: &Context) -> Result<Vec<Item>> {
        let repo = open_repo(ctx)?;

        let head_name = repo
            .head()
            .ok()
            .and_then(|h| h.shorthand().map(|s| s.to_string()));

        let mut items = Vec::new();

        // Local branches
        let local = repo
            .branches(Some(BranchType::Local))
            .map_err(|e| IxError::Provider(format!("git branches: {e}")))?;
        for branch_result in local {
            let (branch, _) =
                branch_result.map_err(|e| IxError::Provider(format!("branch iter: {e}")))?;
            let name = branch
                .name()
                .map_err(|e| IxError::Provider(format!("branch name: {e}")))?
                .unwrap_or("")
                .to_string();
            let is_head = head_name.as_deref() == Some(&name);
            let mut item = Item::new(0, &name, &name).with_group("local");
            if is_head {
                item = item.with_status("*", Category::Positive);
            }
            items.push(item);
        }

        // Remote branches
        let remote = repo
            .branches(Some(BranchType::Remote))
            .map_err(|e| IxError::Provider(format!("git remote branches: {e}")))?;
        for branch_result in remote {
            let (branch, _) =
                branch_result.map_err(|e| IxError::Provider(format!("branch iter: {e}")))?;
            let name = branch
                .name()
                .map_err(|e| IxError::Provider(format!("branch name: {e}")))?
                .unwrap_or("")
                .to_string();
            let item = Item::new(0, &name, &name).with_group("remote");
            items.push(item);
        }

        Ok(items)
    }

    fn preview_cmd(&self, item: &Item) -> Option<String> {
        Some(format!(
            "git log --oneline -15 {}",
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
    fn test_branches_lists_local() {
        let (td, repo) = make_repo();
        make_initial_commit(&repo);
        repo.branch(
            "feature-x",
            &repo.head().unwrap().peel_to_commit().unwrap(),
            false,
        )
        .unwrap();

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
