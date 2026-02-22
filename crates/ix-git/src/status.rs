use ix_core::{Context, Item, Provider, ProviderOption};
use ix_core::error::{IxError, Result};
use git2::{Repository, Status, StatusOptions};

pub struct GitStatusProvider;

impl Provider for GitStatusProvider {
    fn name(&self) -> &str {
        "git-status"
    }

    fn detect(ctx: &Context) -> bool {
        Repository::discover(&ctx.cwd).is_ok()
    }

    fn options() -> Vec<ProviderOption> {
        vec![
            ProviderOption {
                long: "ignored".into(),
                short: None,
                help: "Include gitignored files in the listing".into(),
            },
        ]
    }

    fn list(&self, ctx: &Context) -> Result<Vec<Item>> {
        let repo = Repository::discover(&ctx.cwd)
            .map_err(|e| IxError::Provider(format!("git: {e}")))?;

        let include_ignored = ctx.has_flag("ignored");

        let mut opts = StatusOptions::new();
        opts.include_untracked(true)
            .recurse_untracked_dirs(true)
            .include_ignored(include_ignored);

        let statuses = repo
            .statuses(Some(&mut opts))
            .map_err(|e| IxError::Provider(format!("git statuses: {e}")))?;

        let mut items = Vec::new();
        let mut slot = 1usize;

        // Walk staged changes first
        for entry in statuses.iter() {
            let s = entry.status();
            if !is_staged(s) {
                continue;
            }
            let path = entry.path().unwrap_or("").to_string();
            let status_str = staged_status_str(s);
            let item = Item::new(slot, &path, &path)
                .with_status(status_str)
                .with_group("staged");
            items.push(item);
            slot += 1;
        }

        // Walk unstaged (modified/deleted, not untracked)
        for entry in statuses.iter() {
            let s = entry.status();
            if !is_unstaged(s) {
                continue;
            }
            let path = entry.path().unwrap_or("").to_string();
            let status_str = unstaged_status_str(s);
            let item = Item::new(slot, &path, &path)
                .with_status(status_str)
                .with_group("unstaged");
            items.push(item);
            slot += 1;
        }

        // Walk untracked
        for entry in statuses.iter() {
            let s = entry.status();
            if !s.contains(Status::WT_NEW) {
                continue;
            }
            let path = entry.path().unwrap_or("").to_string();
            let item = Item::new(slot, &path, &path)
                .with_status("??")
                .with_group("untracked");
            items.push(item);
            slot += 1;
        }

        // Walk ignored (only if requested)
        if include_ignored {
            for entry in statuses.iter() {
                let s = entry.status();
                if !s.contains(Status::IGNORED) {
                    continue;
                }
                let path = entry.path().unwrap_or("").to_string();
                let item = Item::new(slot, &path, &path)
                    .with_status("!!")
                    .with_group("ignored");
                items.push(item);
                slot += 1;
            }
        }

        Ok(items)
    }

    fn preview_cmd(&self, item: &Item) -> Option<String> {
        match item.group.as_deref() {
            Some("staged") => Some(format!("git diff --cached -- {}", shell_quote(&item.raw))),
            Some("unstaged") => Some(format!("git diff -- {}", shell_quote(&item.raw))),
            _ => Some(format!("bat --style=plain -- {} 2>/dev/null || cat -- {}", shell_quote(&item.raw), shell_quote(&item.raw))),
        }
    }
}

fn is_staged(s: Status) -> bool {
    s.intersects(
        Status::INDEX_NEW
            | Status::INDEX_MODIFIED
            | Status::INDEX_DELETED
            | Status::INDEX_RENAMED
            | Status::INDEX_TYPECHANGE,
    )
}

fn is_unstaged(s: Status) -> bool {
    s.intersects(
        Status::WT_MODIFIED
            | Status::WT_DELETED
            | Status::WT_TYPECHANGE
            | Status::WT_RENAMED,
    )
}

fn staged_status_str(s: Status) -> &'static str {
    if s.contains(Status::INDEX_NEW) { "A" }
    else if s.contains(Status::INDEX_MODIFIED) { "M" }
    else if s.contains(Status::INDEX_DELETED) { "D" }
    else if s.contains(Status::INDEX_RENAMED) { "R" }
    else if s.contains(Status::INDEX_TYPECHANGE) { "T" }
    else { "?" }
}

fn unstaged_status_str(s: Status) -> &'static str {
    if s.contains(Status::WT_MODIFIED) { "M" }
    else if s.contains(Status::WT_DELETED) { "D" }
    else if s.contains(Status::WT_RENAMED) { "R" }
    else if s.contains(Status::WT_TYPECHANGE) { "T" }
    else { "?" }
}

fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
