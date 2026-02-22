use ix_core::{Context, Item, Provider, ProviderOption};
use ix_core::error::{IxError, Result};
use git2::{Repository, Status, StatusOptions};

pub struct GitStatusProvider;

impl GitStatusProvider {
    pub fn new() -> Self { Self }
}

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

        let workdir = repo.workdir()
            .map(|p| p.to_path_buf())
            .ok_or_else(|| IxError::Provider("bare repository not supported".into()))?;

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
            let rel = entry.path().unwrap_or("").to_string();
            let raw = workdir.join(&rel).to_string_lossy().to_string();
            let status_str = staged_status_str(s);
            let item = Item::new(slot, raw, &rel)
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
            let rel = entry.path().unwrap_or("").to_string();
            let raw = workdir.join(&rel).to_string_lossy().to_string();
            let status_str = unstaged_status_str(s);
            let item = Item::new(slot, raw, &rel)
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
            let rel = entry.path().unwrap_or("").to_string();
            let raw = workdir.join(&rel).to_string_lossy().to_string();
            let item = Item::new(slot, raw, &rel)
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
                let rel = entry.path().unwrap_or("").to_string();
                let raw = workdir.join(&rel).to_string_lossy().to_string();
                let item = Item::new(slot, raw, &rel)
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
            _ => Some(format!("bat --style=plain -- {q} 2>/dev/null || cat -- {q}", q = shell_quote(&item.raw))),
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
    fn test_status_detects_untracked() {
        let (td, _repo) = make_repo();
        std::fs::write(td.path().join("foo.rs"), "fn main() {}").unwrap();

        let items = GitStatusProvider::new().list(&ctx(td.path())).unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].label, "foo.rs");
        assert_eq!(items[0].status.as_deref(), Some("??"));
        assert_eq!(items[0].group.as_deref(), Some("untracked"));
    }

    #[test]
    fn test_status_detects_staged() {
        let (td, repo) = make_repo();
        make_initial_commit(&repo);
        std::fs::write(td.path().join("foo.rs"), "fn main() {}").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new("foo.rs")).unwrap();
        index.write().unwrap();

        let items = GitStatusProvider::new().list(&ctx(td.path())).unwrap();

        assert_eq!(items[0].group.as_deref(), Some("staged"));
        assert_eq!(items[0].status.as_deref(), Some("A"));
    }

    #[test]
    fn test_status_detects_modified_unstaged() {
        let (td, repo) = make_repo();
        std::fs::write(td.path().join("foo.rs"), "original").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("foo.rs")).unwrap();
        index.write().unwrap();
        make_initial_commit(&repo);

        // now modify without staging
        std::fs::write(td.path().join("foo.rs"), "modified").unwrap();

        let items = GitStatusProvider::new().list(&ctx(td.path())).unwrap();

        assert_eq!(items[0].group.as_deref(), Some("unstaged"));
        assert_eq!(items[0].status.as_deref(), Some("M"));
    }

    #[test]
    fn test_status_assigns_sequential_slots() {
        let (td, _repo) = make_repo();
        std::fs::write(td.path().join("a.rs"), "").unwrap();
        std::fs::write(td.path().join("b.rs"), "").unwrap();
        std::fs::write(td.path().join("c.rs"), "").unwrap();

        let items = GitStatusProvider::new().list(&ctx(td.path())).unwrap();

        let slots: Vec<usize> = items.iter().map(|i| i.slot).collect();
        assert_eq!(slots, vec![1, 2, 3]);
    }

    #[test]
    fn test_status_empty_repo() {
        let (td, _repo) = make_repo();
        let items = GitStatusProvider::new().list(&ctx(td.path())).unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn test_status_raw_is_full_path() {
        let (td, _repo) = make_repo();
        std::fs::write(td.path().join("foo.rs"), "").unwrap();

        let items = GitStatusProvider::new().list(&ctx(td.path())).unwrap();

        assert!(items[0].raw.ends_with("foo.rs"));
        assert!(Path::new(&items[0].raw).is_absolute());
    }
}
