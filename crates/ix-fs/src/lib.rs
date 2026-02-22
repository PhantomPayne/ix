use std::fs;
use std::path::PathBuf;

use ix_core::{Context, Item, Provider, ProviderOption};
use ix_core::error::{IxError, Result};

pub struct FsProvider;

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
