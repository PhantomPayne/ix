use std::fs;
use std::path::PathBuf;

use ix_core::error::{IxError, Result};
use ix_core::item::Item;
use ix_core::{Context, Provider};

#[derive(Default)]
pub struct SshProvider;

impl SshProvider {
    pub fn new() -> Self {
        Self
    }
}

/// A parsed SSH host entry.
struct SshHost {
    alias: String,
    hostname: Option<String>,
    user: Option<String>,
    port: Option<String>,
}

impl Provider for SshProvider {
    fn name(&self) -> &str {
        "ssh"
    }

    fn detect(&self, _ctx: &Context) -> bool {
        ssh_config_path().map(|p| p.exists()).unwrap_or(false)
    }

    fn list(&self, _ctx: &Context) -> Result<Vec<Item>> {
        let config_path = ssh_config_path()
            .ok_or_else(|| IxError::Provider("cannot determine home directory".into()))?;

        if !config_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&config_path)
            .map_err(|e| IxError::Provider(format!("read ssh config: {e}")))?;

        let hosts = parse_ssh_config(&content);

        let items = hosts
            .into_iter()
            .map(|h| {
                let label = build_label(&h);
                Item::new(0, &h.alias, label)
            })
            .collect();

        Ok(items)
    }

    fn preview_cmd(&self, item: &Item) -> Option<String> {
        Some(format!("ssh -G {} | head -20", item.raw))
    }
}

fn ssh_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".ssh").join("config"))
}

fn build_label(host: &SshHost) -> String {
    let mut parts = vec![host.alias.clone()];

    let target = match (&host.user, &host.hostname, &host.port) {
        (Some(user), Some(hostname), Some(port)) => {
            format!("{user}@{hostname}:{port}")
        }
        (Some(user), Some(hostname), None) => format!("{user}@{hostname}"),
        (None, Some(hostname), Some(port)) => format!("{hostname}:{port}"),
        (None, Some(hostname), None) => hostname.clone(),
        _ => return host.alias.clone(),
    };

    parts.push("→".into());
    parts.push(target);
    parts.join(" ")
}

/// Parse an SSH config file into a list of host entries.
///
/// Skips wildcard entries (`*`) and `Match` blocks.
fn parse_ssh_config(content: &str) -> Vec<SshHost> {
    let mut hosts = Vec::new();
    let mut current: Option<SshHost> = None;

    for line in content.lines() {
        let line = line.trim();

        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Split on first whitespace or '='
        let (key, value) = match line.split_once(|c: char| c.is_whitespace() || c == '=') {
            Some((k, v)) => (k.trim(), v.trim()),
            None => continue,
        };

        match key.to_lowercase().as_str() {
            "host" => {
                // Finish previous entry
                if let Some(h) = current.take() {
                    hosts.push(h);
                }

                // Parse host aliases — skip wildcards
                for alias in value.split_whitespace() {
                    if alias.contains('*') || alias.contains('?') {
                        continue;
                    }
                    current = Some(SshHost {
                        alias: alias.to_string(),
                        hostname: None,
                        user: None,
                        port: None,
                    });
                    // Only take the first non-wildcard alias
                    break;
                }
            }
            "hostname" => {
                if let Some(ref mut h) = current {
                    h.hostname = Some(value.to_string());
                }
            }
            "user" => {
                if let Some(ref mut h) = current {
                    h.user = Some(value.to_string());
                }
            }
            "port" => {
                if let Some(ref mut h) = current {
                    h.port = Some(value.to_string());
                }
            }
            _ => {}
        }
    }

    // Don't forget the last entry
    if let Some(h) = current {
        hosts.push(h);
    }

    hosts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_config() {
        let config = r#"
Host prod
    HostName 10.0.0.1
    User admin
    Port 2222

Host staging
    HostName staging.example.com
    User deploy
"#;
        let hosts = parse_ssh_config(config);
        assert_eq!(hosts.len(), 2);
        assert_eq!(hosts[0].alias, "prod");
        assert_eq!(hosts[0].hostname.as_deref(), Some("10.0.0.1"));
        assert_eq!(hosts[0].user.as_deref(), Some("admin"));
        assert_eq!(hosts[0].port.as_deref(), Some("2222"));
        assert_eq!(hosts[1].alias, "staging");
        assert_eq!(hosts[1].hostname.as_deref(), Some("staging.example.com"));
    }

    #[test]
    fn test_parse_skips_wildcards() {
        let config = r#"
Host *
    ServerAliveInterval 60

Host myserver
    HostName example.com
"#;
        let hosts = parse_ssh_config(config);
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].alias, "myserver");
    }

    #[test]
    fn test_parse_handles_equals_syntax() {
        let config = "Host box\nHostName=10.0.0.5\nUser=root\n";
        let hosts = parse_ssh_config(config);
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].hostname.as_deref(), Some("10.0.0.5"));
        assert_eq!(hosts[0].user.as_deref(), Some("root"));
    }

    #[test]
    fn test_build_label_full() {
        let h = SshHost {
            alias: "prod".into(),
            hostname: Some("10.0.0.1".into()),
            user: Some("admin".into()),
            port: Some("2222".into()),
        };
        assert_eq!(build_label(&h), "prod → admin@10.0.0.1:2222");
    }

    #[test]
    fn test_build_label_no_port() {
        let h = SshHost {
            alias: "dev".into(),
            hostname: Some("dev.example.com".into()),
            user: Some("tom".into()),
            port: None,
        };
        assert_eq!(build_label(&h), "dev → tom@dev.example.com");
    }

    #[test]
    fn test_build_label_no_extras() {
        let h = SshHost {
            alias: "simple".into(),
            hostname: None,
            user: None,
            port: None,
        };
        assert_eq!(build_label(&h), "simple");
    }

    #[test]
    fn test_parse_comments_and_empty_lines() {
        let config = r#"
# This is a comment

Host myhost
    # Another comment
    HostName example.com

"#;
        let hosts = parse_ssh_config(config);
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].alias, "myhost");
    }
}
