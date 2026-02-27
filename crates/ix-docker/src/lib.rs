use std::collections::HashMap;

use bollard::container::ListContainersOptions;
use bollard::Docker;
use ix_core::error::{IxError, Result};
use ix_core::item::{Category, Item};
use ix_core::{Context, Provider};
use shell_escape::escape;
use std::borrow::Cow;

#[derive(Default)]
pub struct DockerProvider;

impl DockerProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Provider for DockerProvider {
    fn name(&self) -> &str {
        "docker"
    }

    fn detect(&self, _ctx: &Context) -> bool {
        std::path::Path::new("/var/run/docker.sock").exists()
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

            docker
                .list_containers(Some(opts))
                .await
                .map_err(|e| IxError::Provider(format!("list containers: {e}")))
        })?;

        let mut items = Vec::new();

        for container in containers {
            // Prefer name over ID for readability
            let name = container
                .names
                .as_ref()
                .and_then(|ns| ns.first())
                .map(|n| n.trim_start_matches('/').to_string())
                .unwrap_or_else(|| {
                    container
                        .id
                        .as_deref()
                        .unwrap_or("unknown")
                        .chars()
                        .take(12)
                        .collect()
                });

            let image = container.image.as_deref().unwrap_or("unknown");
            let label = format!("{name} ({image})");

            let state = container.state.as_deref().unwrap_or("unknown");
            let status_str = state;

            let group = if state == "running" {
                "running"
            } else {
                "stopped"
            };

            let category = match state {
                "running" => Category::Positive,
                "paused" => Category::Warning,
                "exited" => Category::Neutral,
                "dead" | "restarting" => Category::Negative,
                _ => Category::Unknown,
            };

            let item = Item::new(0, &name, &label)
                .with_status(status_str, category)
                .with_group(group);
            items.push(item);
        }

        Ok(items)
    }

    fn preview_cmd(&self, item: &Item) -> Option<String> {
        Some(format!(
            "docker logs --tail 30 {}",
            escape(Cow::Borrowed(&item.raw))
        ))
    }
}

#[cfg(test)]
use ix_core::test_utils::ctx_default;

#[cfg(test)]
mod tests {
    use super::*;
    use bollard::container::{Config, CreateContainerOptions, RemoveContainerOptions};
    use bollard::models::HostConfig;

    async fn setup_container(docker: &Docker, name: &str) {
        let _ = docker
            .remove_container(
                name,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await;
        docker
            .create_container(
                Some(CreateContainerOptions {
                    name,
                    platform: None,
                }),
                Config {
                    image: Some("alpine:latest"),
                    cmd: Some(vec!["sleep", "30"]),
                    host_config: Some(HostConfig {
                        auto_remove: Some(false),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        docker.start_container::<String>(name, None).await.unwrap();
    }

    async fn teardown_container(docker: &Docker, name: &str) {
        docker.stop_container(name, None).await.ok();
        docker
            .remove_container(
                name,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await
            .ok();
    }

    #[tokio::test]
    #[ignore = "requires docker daemon"]
    async fn test_docker_lists_running_container() {
        let docker = Docker::connect_with_local_defaults().unwrap();
        let name = "ix-test-running";
        setup_container(&docker, name).await;

        let items = DockerProvider::new().list(&ctx_default()).unwrap();
        assert!(items.iter().any(|i| i.label.contains(name)));

        teardown_container(&docker, name).await;
    }

    #[tokio::test]
    #[ignore = "requires docker daemon"]
    async fn test_docker_hides_stopped_by_default() {
        let docker = Docker::connect_with_local_defaults().unwrap();
        let name = "ix-test-stopped";
        setup_container(&docker, name).await;
        docker.stop_container(name, None).await.unwrap();

        let items = DockerProvider::new().list(&ctx_default()).unwrap();
        assert!(!items.iter().any(|i| i.label.contains(name)));

        teardown_container(&docker, name).await;
    }

    #[tokio::test]
    #[ignore = "requires docker daemon"]
    async fn test_docker_shows_stopped_with_all_flag() {
        let docker = Docker::connect_with_local_defaults().unwrap();
        let name = "ix-test-stopped-all";
        setup_container(&docker, name).await;
        docker.stop_container(name, None).await.unwrap();

        let mut c = ctx_default();
        c.flags.insert("all".into());
        let items = DockerProvider::new().list(&c).unwrap();
        assert!(items.iter().any(|i| i.label.contains(name)));

        teardown_container(&docker, name).await;
    }

    #[tokio::test]
    #[ignore = "requires docker daemon"]
    async fn test_docker_raw_is_container_name() {
        let docker = Docker::connect_with_local_defaults().unwrap();
        let name = "ix-test-raw";
        setup_container(&docker, name).await;

        let items = DockerProvider::new().list(&ctx_default()).unwrap();
        let item = items.iter().find(|i| i.label.contains(name)).unwrap();

        // raw must be the name not the ID — names are human readable
        assert_eq!(item.raw, name);

        teardown_container(&docker, name).await;
    }
}
