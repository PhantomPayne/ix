use crate::{detect_git, open_repo};
use git2::{Status, StatusOptions};
use ix_core::error::{IxError, Result};
use ix_core::item::{Category, Item};
use ix_core::{Context, Provider};
use shell_escape::escape;
use std::borrow::Cow;

#[derive(Default)]
pub struct GitStatusProvider;

impl GitStatusProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Provider for GitStatusProvider {
    fn name(&self) -> &str {
        "git-status"
    }

    fn detect(&self, ctx: &Context) -> bool {
        detect_git(ctx)
    }

    fn list(&self, ctx: &Context) -> Result<Vec<Item>> {
        let repo = open_repo(ctx)?;

        let workdir = repo
            .workdir()
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

        let mut staged = Vec::new();
        let mut unstaged = Vec::new();
        let mut untracked = Vec::new();
        let mut ignored = Vec::new();

        for entry in statuses.iter() {
            let s = entry.status();
            let rel = entry.path().unwrap_or("").to_string();
            let raw = workdir.join(&rel).to_string_lossy().to_string();

            if is_staged(s) {
                let (status_str, category) = staged_status_str(s);
                staged.push(
                    Item::new(0, &raw, &rel)
                        .with_status(status_str, category)
                        .with_group("staged"),
                );
            }
            if is_unstaged(s) {
                let (status_str, category) = unstaged_status_str(s);
                unstaged.push(
                    Item::new(0, &raw, &rel)
                        .with_status(status_str, category)
                        .with_group("unstaged"),
                );
            }
            if s.contains(Status::WT_NEW) {
                untracked.push(
                    Item::new(0, &raw, &rel)
                        .with_status("??", Category::Neutral)
                        .with_group("untracked"),
                );
            }
            if include_ignored && s.contains(Status::IGNORED) {
                ignored.push(
                    Item::new(0, &raw, &rel)
                        .with_status("!!", Category::Neutral)
                        .with_group("ignored"),
                );
            }
        }

        let mut items =
            Vec::with_capacity(staged.len() + unstaged.len() + untracked.len() + ignored.len());
        items.extend(staged);
        items.extend(unstaged);
        items.extend(untracked);
        items.extend(ignored);

        Ok(items)
    }

    fn preview_cmd(&self, item: &Item) -> Option<String> {
        match item.group.as_deref() {
            Some("staged") => Some(format!(
                "git diff --cached -- {}",
                escape(Cow::Borrowed(&item.raw))
            )),
            Some("unstaged") => Some(format!("git diff -- {}", escape(Cow::Borrowed(&item.raw)))),
            _ => Some(format!(
                "bat --style=plain -- {q} 2>/dev/null || cat -- {q}",
                q = escape(Cow::Borrowed(&item.raw))
            )),
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
        Status::WT_MODIFIED | Status::WT_DELETED | Status::WT_TYPECHANGE | Status::WT_RENAMED,
    )
}

fn staged_status_str(s: Status) -> (&'static str, Category) {
    if s.contains(Status::INDEX_NEW) {
        ("A", Category::Positive)
    } else if s.contains(Status::INDEX_MODIFIED) {
        ("M", Category::Warning)
    } else if s.contains(Status::INDEX_DELETED) {
        ("D", Category::Negative)
    } else if s.contains(Status::INDEX_RENAMED) {
        ("R", Category::Positive)
    } else if s.contains(Status::INDEX_TYPECHANGE) {
        ("T", Category::Warning)
    } else {
        ("?", Category::Unknown)
    }
}

fn unstaged_status_str(s: Status) -> (&'static str, Category) {
    if s.contains(Status::WT_MODIFIED) {
        ("M", Category::Warning)
    } else if s.contains(Status::WT_DELETED) {
        ("D", Category::Negative)
    } else if s.contains(Status::WT_RENAMED) {
        ("R", Category::Positive)
    } else if s.contains(Status::WT_TYPECHANGE) {
        ("T", Category::Warning)
    } else {
        ("?", Category::Unknown)
    }
}

#[cfg(test)]
use ix_core::test_utils::ctx_path as ctx;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{make_initial_commit, make_repo};
    use std::path::Path;

    #[test]
    fn test_status_detects_untracked() {
        let (td, _repo) = make_repo();
        std::fs::write(td.path().join("foo.rs"), "fn main() {}").unwrap();

        let items = GitStatusProvider::new().list(&ctx(td.path())).unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].label, "foo.rs");
        assert_eq!(items[0].status.as_ref().map(|s| s.as_str()), Some("??"));
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
        assert_eq!(items[0].status.as_ref().map(|s| s.as_str()), Some("A"));
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
        assert_eq!(items[0].status.as_ref().map(|s| s.as_str()), Some("M"));
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
