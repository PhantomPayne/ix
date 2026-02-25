use crate::{detect_git, open_repo};
use ix_core::error::{IxError, Result};
use ix_core::item::{Category, Item};
use ix_core::{Context, Provider};
use std::path::Path;

#[derive(Default)]
pub struct GitWorktreeProvider;

impl GitWorktreeProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Provider for GitWorktreeProvider {
    fn name(&self) -> &str {
        "git-worktrees"
    }

    fn detect(&self, ctx: &Context) -> bool {
        detect_git(ctx)
    }

    fn list(&self, ctx: &Context) -> Result<Vec<Item>> {
        let repo = open_repo(ctx)?;

        let worktree_names = repo
            .worktrees()
            .map_err(|e| IxError::Provider(format!("git worktrees: {e}")))?;

        let mut items = Vec::new();

        // Always include the main repository (it is a worktree too, conceptually)
        let main_path = repo.workdir().unwrap_or_else(|| repo.path()).to_path_buf();
        let main_branch = head_shorthand(&repo);
        
        items.push(create_worktree_item(
            &main_path,
            &main_branch.unwrap_or_else(|| String::from("HEAD")),
            ctx,
        ));

        for name_opt in worktree_names.iter() {
            if let Some(name) = name_opt {
                let wt = repo
                    .find_worktree(name)
                    .map_err(|e| IxError::Provider(format!("git find_worktree {name}: {e}")))?;

                let wt_repo = git2::Repository::open_from_worktree(&wt)
                    .map_err(|e| IxError::Provider(format!("git open_from_worktree {name}: {e}")))?;

                let branch = head_shorthand(&wt_repo);
                
                let wt_path = wt.path().to_path_buf();

                items.push(create_worktree_item(
                    &wt_path,
                    &branch.unwrap_or_else(|| name.to_string()),
                    ctx,
                ));
            }
        }

        Ok(items)
    }

    fn preview_cmd(&self, item: &Item) -> Option<String> {
        // We could run `git status` in the worktree directory if we want, or `git log`
        Some(format!(
            "git -C \"{}\" log --oneline -15",
            item.raw
        ))
    }
}

fn head_shorthand(repo: &git2::Repository) -> Option<String> {
    repo.head()
        .ok()
        .and_then(|h| h.shorthand().map(|s| s.to_string()))
}

fn create_worktree_item(path: &Path, branch_name: &str, ctx: &Context) -> Item {
    let path_str = path.to_string_lossy().to_string();
    
    // Format label to show branch name clearly
    let label = format!("{branch_name}");
    
    // Check if this is the current directory's worktree
    let is_current = is_path_in_dir(&ctx.cwd, path);
    
    let mut item = Item::new(0, &path_str, &label)
        .with_group("worktrees")
        .with_status(&path_str, Category::Neutral);
        
    if is_current {
        item = item.with_status("*", Category::Positive);
    }
    
    item
}

fn is_path_in_dir(dir: &Path, target: &Path) -> bool {
    dir.starts_with(target)
}

#[cfg(test)]
use ix_core::test_utils::ctx_path as ctx;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{make_initial_commit, make_repo};

    #[test]
    fn test_worktrees_lists_main() {
        let (td, repo) = make_repo();
        make_initial_commit(&repo);

        let items = GitWorktreeProvider::new().list(&ctx(td.path())).unwrap();

        assert_eq!(items.len(), 1);
        assert!(items[0].label.contains("master") || items[0].label.contains("main"));
        // raw should have the path
        assert!(items[0].raw.contains(td.path().to_str().unwrap()));
    }
}
