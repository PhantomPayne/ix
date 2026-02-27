use std::borrow::Cow;

use ignore::WalkBuilder;
use ix_core::error::{IxError, Result};
use ix_core::item::Item;
use ix_core::{Context, Provider};
use shell_escape::escape;

#[derive(Default)]
pub struct FdProvider;

impl FdProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Provider for FdProvider {
    fn name(&self) -> &str {
        "fd"
    }

    fn detect(&self, _ctx: &Context) -> bool {
        true // always available
    }

    fn list(&self, ctx: &Context) -> Result<Vec<Item>> {
        let show_hidden = ctx.has_flag("a") || ctx.has_flag("hidden");
        let show_all = ctx.has_flag("A") || ctx.has_flag("all");
        let show_hidden = show_hidden || show_all;

        let mut builder = WalkBuilder::new(&ctx.cwd);
        builder.hidden(!show_hidden);
        builder.git_ignore(!show_all);

        let mut items = Vec::new();

        for result in builder.build() {
            let entry = result.map_err(|e| IxError::Provider(format!("ignore: {e}")))?;
            let path = entry.path();

            // Skip the base directory itself
            if path == ctx.cwd {
                continue;
            }

            let file_type = entry.file_type();
            let is_dir = file_type.map(|ft| ft.is_dir()).unwrap_or(false);

            // Calculate label relative to cwd for display
            let label = if let Ok(stripped) = path.strip_prefix(&ctx.cwd) {
                stripped.to_string_lossy().to_string()
            } else {
                path.to_string_lossy().to_string()
            };

            let raw = path.to_string_lossy().to_string();

            let group = if is_dir { "dirs" } else { "files" };
            items.push(Item::new(0, raw, &label).with_group(group));
        }

        // Sort alphabetically
        items.sort_unstable_by(|a, b| a.group.cmp(&b.group).then_with(|| a.label.cmp(&b.label)));

        Ok(items)
    }

    fn preview_cmd(&self, item: &Item) -> Option<String> {
        let path = &item.raw;
        if item.group.as_deref() == Some("dirs") {
            Some(format!("ls -la {}", escape(Cow::Borrowed(path))))
        } else {
            Some(format!(
                "bat --style=plain -- {q} 2>/dev/null || cat -- {q}",
                q = escape(Cow::Borrowed(path))
            ))
        }
    }
}
