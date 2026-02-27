use ix_core::error::Result;
use ix_core::item::Item;
use ix_core::{Context, Provider};

#[derive(Default)]
pub struct EnvProvider;

impl EnvProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Provider for EnvProvider {
    fn name(&self) -> &str {
        "env"
    }

    fn detect(&self, _ctx: &Context) -> bool {
        true
    }

    fn list(&self, _ctx: &Context) -> Result<Vec<Item>> {
        let mut pairs: Vec<(String, String)> = std::env::vars().collect();
        pairs.sort_unstable_by(|a, b| a.0.cmp(&b.0));

        let items = pairs
            .into_iter()
            .map(|(key, value)| {
                let truncated = if value.chars().count() > 60 {
                    format!("{}…", value.chars().take(59).collect::<String>())
                } else {
                    value.clone()
                };
                let label = format!("{key}={truncated}");
                Item::new(0, &key, label)
            })
            .collect();

        Ok(items)
    }

    fn preview_cmd(&self, item: &Item) -> Option<String> {
        Some(format!("printenv {}", item.raw))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_core::test_utils::ctx_default;

    #[test]
    fn test_env_lists_path() {
        let items = EnvProvider::new().list(&ctx_default()).unwrap();
        assert!(items.iter().any(|i| i.raw == "PATH"));
    }

    #[test]
    fn test_env_raw_is_key() {
        let items = EnvProvider::new().list(&ctx_default()).unwrap();
        let home = items.iter().find(|i| i.raw == "HOME");
        assert!(home.is_some(), "HOME variable should be in the list");
    }

    #[test]
    fn test_env_label_contains_key_and_value() {
        let items = EnvProvider::new().list(&ctx_default()).unwrap();
        let home = items.iter().find(|i| i.raw == "HOME").unwrap();
        assert!(home.label.starts_with("HOME="));
    }

    #[test]
    fn test_env_items_sorted() {
        let items = EnvProvider::new().list(&ctx_default()).unwrap();
        let keys: Vec<&str> = items.iter().map(|i| i.raw.as_str()).collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted);
    }
}
