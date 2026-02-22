use ix_core::{Context, Item, Provider, ProviderOption};
use ix_core::error::{IxError, Result};
use bollard::Docker;
use bollard::container::{ListContainersOptions};
use std::collections::HashMap;

pub struct DockerProvider;

impl Provider for DockerProvider {
    fn name(&self) -> &str {
        "docker"
    }

    fn detect(_ctx: &Context) -> bool {
        // Try to connect to Docker; if it fails, provider is unavailable
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map(|rt| rt.block_on(async { Docker::connect_with_local_defaults().is_ok() }))
            .unwrap_or(false)
    }

    fn options() -> Vec<ProviderOption> {
        vec![
            ProviderOption {
                long: "all".into(),
                short: Some('a'),
                help: "Include stopped containers (default: running only)".into(),
            },
        ]
    }

    fn list(&self, ctx: &Context) -> Result<Vec<Item>> {
        let show_all = ctx.has_flag("a") || ctx.has_flag("all");

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| IxError::Provider(format!("tokio runtime: {e}")))?;

        let containers = rt.block_on(async {
            let docker = Docker::connect_with_local_defaults()
                .map_err(|e| IxError::Provider(format!("docker connect: {e}")))?;

            let mut filters: HashMap<&str, Vec<&str>> = HashMap::new();
            if !show_all {
                filters.insert("status", vec!["running"]);
            }

            let opts = ListContainersOptions {
                all: show_all,
                filters,
                ..Default::default()
            };

            docker.list_containers(Some(opts)).await
                .map_err(|e| IxError::Provider(format!("list containers: {e}")))
        })?;

        let mut items = Vec::new();
        let mut slot = 1usize;

        for container in containers {
            // Prefer name over ID for readability
            let name = container.names
                .as_ref()
                .and_then(|ns| ns.first())
                .map(|n| n.trim_start_matches('/').to_string())
                .unwrap_or_else(|| {
                    container.id.as_deref().unwrap_or("unknown").chars().take(12).collect()
                });

            let image = container.image.as_deref().unwrap_or("unknown");
            let label = format!("{name} ({image})");

            let state = container.state.as_deref().unwrap_or("unknown");
            let status_str = match state {
                "running" => "running",
                "exited" => "exited",
                "paused" => "paused",
                "created" => "created",
                _ => state,
            };

            let group = if state == "running" { "running" } else { "stopped" };

            let item = Item::new(slot, &name, &label)
                .with_status(status_str)
                .with_group(group);
            items.push(item);
            slot += 1;
        }

        Ok(items)
    }

    fn preview_cmd(&self, item: &Item) -> Option<String> {
        Some(format!("docker logs --tail 30 {}", shell_quote(&item.raw)))
    }
}

fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
