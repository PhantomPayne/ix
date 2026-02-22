use ix_core::{Context, Item, Provider, ProviderOption};
use ix_core::error::{IxError, Result};
use git2::Repository;

pub struct GitStashProvider;

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
