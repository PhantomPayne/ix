use std::fs;
use std::path::PathBuf;

use ix_core::{Context, Item, Provider, ProviderOption};
use ix_core::error::{IxError, Result};

pub struct FsProvider;

impl FsProvider {
    pub fn new() -> Self { Self }
}

impl Default for FsProvider {
    fn default() -> Self { Self::new() }
}

impl Provider for FsProvider {
    fn name(&self) -> &str {
        "ls"
    }

    fn detect(_ctx: &Context) -> bool {
        true // always available
    }

    fn options() -> Vec<ProviderOption> {
        vec![
            ProviderOption {
                long: "hidden".into(),
                short: Some('a'),
                help: "Include hidden files (dotfiles)".into(),
            },
            ProviderOption {
                long: "all".into(),
                short: Some('A'),
                help: "Include hidden files AND gitignored files".into(),
            },
        ]
    }

    fn list(&self, ctx: &Context) -> Result<Vec<Item>> {
        let show_hidden = ctx.has_flag("a") || ctx.has_flag("hidden");
        let show_all = ctx.has_flag("A") || ctx.has_flag("all");
        // --all implies --hidden
        let show_hidden = show_hidden || show_all;

        // When not showing all, try to respect gitignore rules via git2
        let repo = if !show_all {
            git2::Repository::discover(&ctx.cwd).ok()
        } else {
            None
        };

        let read_dir = fs::read_dir(&ctx.cwd)
            .map_err(|e| IxError::Provider(format!("read_dir: {e}")))?;

        let mut dirs: Vec<(String, PathBuf)> = Vec::new();
        let mut files: Vec<(String, PathBuf)> = Vec::new();

        for entry_result in read_dir {
            let entry = entry_result
                .map_err(|e| IxError::Provider(format!("dir entry: {e}")))?;
            let name = entry.file_name().to_string_lossy().to_string();

            // Filter hidden files unless requested
            if !show_hidden && name.starts_with('.') {
                continue;
            }

            let path = entry.path();

            // Filter gitignored files unless --all
            if let Some(ref repo) = repo {
                if repo.is_path_ignored(&path).unwrap_or(false) {
                    continue;
                }
            }

            let file_type = entry.file_type()
                .map_err(|e| IxError::Provider(format!("file type: {e}")))?;

            if file_type.is_dir() {
                dirs.push((name, path));
            } else {
                files.push((name, path));
            }
        }

        // Sort alphabetically
        dirs.sort_by(|a, b| a.0.cmp(&b.0));
        files.sort_by(|a, b| a.0.cmp(&b.0));

        let mut items = Vec::new();
        let mut slot = 1usize;

        for (name, path) in dirs {
            let raw = path.to_string_lossy().to_string();
            let item = Item::new(slot, raw, &name)
                .with_group("dirs");
            items.push(item);
            slot += 1;
        }

        for (name, path) in files {
            let raw = path.to_string_lossy().to_string();
            let item = Item::new(slot, raw, &name)
                .with_group("files");
            items.push(item);
            slot += 1;
        }

        Ok(items)
    }

    fn preview_cmd(&self, item: &Item) -> Option<String> {
        let path = &item.raw;
        if item.group.as_deref() == Some("dirs") {
            Some(format!("ls -la {}", shell_quote(path)))
        } else {
            Some(format!(
                "bat --style=plain -- {q} 2>/dev/null || cat -- {q}",
                q = shell_quote(path)
            ))
        }
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

    fn ctx(path: &Path) -> Context {
        Context::new(path.to_path_buf())
    }

    #[test]
    fn test_fs_lists_files() {
        let td = tempdir().unwrap();
        std::fs::write(td.path().join("main.rs"), "").unwrap();
        std::fs::write(td.path().join("lib.rs"), "").unwrap();

        let items = FsProvider::new().list(&ctx(td.path())).unwrap();

        assert_eq!(items.len(), 2);
        assert!(items.iter().any(|i| i.label == "main.rs"));
        assert!(items.iter().any(|i| i.label == "lib.rs"));
    }

    #[test]
    fn test_fs_hides_dotfiles_by_default() {
        let td = tempdir().unwrap();
        std::fs::write(td.path().join("main.rs"), "").unwrap();
        std::fs::write(td.path().join(".hidden"), "").unwrap();

        let items = FsProvider::new().list(&ctx(td.path())).unwrap();

        assert!(items.iter().any(|i| i.label == "main.rs"));
        assert!(!items.iter().any(|i| i.label == ".hidden"));
    }

    #[test]
    fn test_fs_shows_dotfiles_with_hidden_flag() {
        let td = tempdir().unwrap();
        std::fs::write(td.path().join(".hidden"), "").unwrap();

        let mut c = ctx(td.path());
        c.flags.insert("hidden".into(), "".into());
        let items = FsProvider::new().list(&c).unwrap();

        assert!(items.iter().any(|i| i.label == ".hidden"));
    }

    #[test]
    fn test_fs_respects_gitignore_by_default() {
        let td = tempdir().unwrap();
        // init a git repo so .gitignore is respected
        git2::Repository::init(td.path()).unwrap();
        std::fs::write(td.path().join(".gitignore"), "*.log\n").unwrap();
        std::fs::write(td.path().join("main.rs"), "").unwrap();
        std::fs::write(td.path().join("debug.log"), "").unwrap();

        let items = FsProvider::new().list(&ctx(td.path())).unwrap();

        assert!(items.iter().any(|i| i.label == "main.rs"));
        assert!(!items.iter().any(|i| i.label == "debug.log"));
    }

    #[test]
    fn test_fs_shows_gitignored_with_all_flag() {
        let td = tempdir().unwrap();
        git2::Repository::init(td.path()).unwrap();
        std::fs::write(td.path().join(".gitignore"), "*.log\n").unwrap();
        std::fs::write(td.path().join("debug.log"), "").unwrap();

        let mut c = ctx(td.path());
        c.flags.insert("all".into(), "".into());
        let items = FsProvider::new().list(&c).unwrap();

        assert!(items.iter().any(|i| i.label == "debug.log"));
    }

    #[test]
    fn test_fs_groups_dirs_and_files() {
        let td = tempdir().unwrap();
        std::fs::create_dir(td.path().join("subdir")).unwrap();
        std::fs::write(td.path().join("main.rs"), "").unwrap();

        let items = FsProvider::new().list(&ctx(td.path())).unwrap();

        let dir = items.iter().find(|i| i.label == "subdir").unwrap();
        let file = items.iter().find(|i| i.label == "main.rs").unwrap();
        assert_eq!(dir.group.as_deref(), Some("dirs"));
        assert_eq!(file.group.as_deref(), Some("files"));
    }

    #[test]
    fn test_fs_raw_is_absolute_path() {
        let td = tempdir().unwrap();
        std::fs::write(td.path().join("main.rs"), "").unwrap();

        let items = FsProvider::new().list(&ctx(td.path())).unwrap();

        assert!(Path::new(&items[0].raw).is_absolute());
    }
}
