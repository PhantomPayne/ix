use std::collections::HashMap;
use std::path::PathBuf;

use crate::item::Item;
use crate::error::Result;

#[derive(Debug, Clone)]
pub struct Context {
    pub cwd: PathBuf,
    pub env: HashMap<String, String>,
    /// User-passed flags, e.g. {"a": "", "all": "", "hidden": ""}
    pub flags: HashMap<String, String>,
}

impl Context {
    pub fn new(cwd: PathBuf) -> Self {
        Self {
            cwd,
            env: std::env::vars().collect(),
            flags: HashMap::new(),
        }
    }

    pub fn with_flags(mut self, flags: HashMap<String, String>) -> Self {
        self.flags = flags;
        self
    }

    pub fn has_flag(&self, name: &str) -> bool {
        self.flags.contains_key(name)
    }
}

#[derive(Debug, Clone)]
pub struct ProviderOption {
    pub long: String,
    pub short: Option<char>,
    pub help: String,
}

pub trait Provider {
    fn name(&self) -> &str;
    fn detect(ctx: &Context) -> bool where Self: Sized;
    fn options() -> Vec<ProviderOption> where Self: Sized { vec![] }

    fn list(&self, ctx: &Context) -> Result<Vec<Item>>;
    /// Returns a shell command string for the preview pane (side-effect free).
    fn preview_cmd(&self, item: &Item) -> Option<String>;
}
