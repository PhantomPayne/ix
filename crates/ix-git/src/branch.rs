use ix_core::{Context, Item, Provider, ProviderOption};
use ix_core::error::{IxError, Result};
use git2::{BranchType, Repository};

pub struct GitBranchProvider;

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
