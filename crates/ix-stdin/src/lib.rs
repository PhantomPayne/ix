use std::io::{self, BufRead};

use ix_core::item::Item;
use ix_core::provider::{Context, Provider};

pub struct StdinProvider;

impl StdinProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for StdinProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl Provider for StdinProvider {
    fn name(&self) -> &str {
        "stdin"
    }

    fn detect(&self, _ctx: &Context) -> bool {
        // we use an external crate `is-terminal` or rustix in ix-cli to register it
        // detect() is used for auto-detecting the provider in the cli.
        // we'll handle `ix` piped from stdin in main.rs instead of using `detect`
        false
    }

    fn list(&self, _ctx: &Context) -> ix_core::error::Result<Vec<Item>> {
        let stdin = io::stdin();
        let mut items = Vec::new();
        let mut slot = 1;

        for line in stdin.lock().lines() {
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
