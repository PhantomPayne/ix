use std::collections::HashSet;
use std::path::PathBuf;

use crate::error::Result;
use crate::item::Item;

#[derive(Debug, Clone)]
pub struct Context {
    pub cwd: PathBuf,
    /// User-passed flags, e.g. {"a", "all", "hidden"}
    pub flags: HashSet<String>,
}

impl Context {
    pub fn new(cwd: PathBuf) -> Self {
        Self {
            cwd,
            flags: HashSet::new(),
        }
    }

    pub fn with_flags(mut self, flags: HashSet<String>) -> Self {
        self.flags = flags;
        self
    }

    pub fn has_flag(&self, name: &str) -> bool {
        self.flags.contains(name)
    }
}

pub trait Provider {
    fn name(&self) -> &str;
    fn detect(&self, _ctx: &Context) -> bool {
        false
    }

    fn list(&self, ctx: &Context) -> Result<Vec<Item>>;
    /// Returns a shell command string for the preview pane (side-effect free).
    /// Returns `None` by default; providers override this when they have a useful preview.
    fn preview_cmd(&self, _item: &Item) -> Option<String> {
        None
    }
}
