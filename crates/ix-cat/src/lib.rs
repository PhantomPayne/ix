use std::fs::File;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};

use ix_core::item::Item;
use ix_core::provider::{Context, Provider};

pub struct CatProvider {
    file: PathBuf,
}

impl CatProvider {
    pub fn new<P: AsRef<Path>>(file: P) -> Self {
        Self {
            file: file.as_ref().to_path_buf(),
        }
    }
}

impl Provider for CatProvider {
    fn name(&self) -> &str {
        "cat"
    }

    fn list(&self, ctx: &Context) -> ix_core::error::Result<Vec<Item>> {
        // Resolve target file path relative to context cwd
        let path = if self.file.is_absolute() {
            self.file.clone()
        } else {
            ctx.cwd.join(&self.file)
        };

        if !path.exists() {
            return Err(ix_core::error::IxError::Provider(format!(
                "File not found: {}",
                path.display()
            )));
        }

        let file = File::open(&path).map_err(|e| {
            ix_core::error::IxError::Provider(format!(
                "Failed to open file '{}': {e}",
                path.display()
            ))
        })?;
        let reader = io::BufReader::new(file);

        let mut items = Vec::new();
        let mut slot = 1;

        for line in reader.lines() {
            let line = line.map_err(|e| ix_core::error::IxError::Provider(e.to_string()))?;
            if line.trim().is_empty() {
                continue;
            }
            items.push(Item::new(slot, &line, &line));
            slot += 1;
        }

        Ok(items)
    }

    fn preview_cmd(&self, _item: &Item) -> Option<String> {
        None
    }
}
