use ix_core::{Context, Item, Provider, ProviderOption};
use ix_core::error::Result;
use sysinfo::{ProcessStatus, System, Users};

pub struct ProcessProvider;

impl Provider for ProcessProvider {
    fn name(&self) -> &str {
        "ps"
    }

    fn detect(_ctx: &Context) -> bool {
        true // always available
    }

    fn options() -> Vec<ProviderOption> {
        vec![
            ProviderOption {
                long: "all".into(),
                short: Some('a'),
                help: "Show all users' processes, not just current user".into(),
            },
        ]
    }

    fn list(&self, ctx: &Context) -> Result<Vec<Item>> {
        let show_all = ctx.has_flag("a") || ctx.has_flag("all");

        let mut sys = System::new_all();
        sys.refresh_all();

        let users = Users::new_with_refreshed_list();

        // Get current user name from environment
        let current_uid = std::env::var("USER")
            .or_else(|_| std::env::var("LOGNAME"))
            .ok();

        let mut items = Vec::new();
        let mut slot = 1usize;

        let mut procs: Vec<_> = sys.processes().values().collect();
        // Sort by PID for stable ordering
        procs.sort_by_key(|p| p.pid());

        for process in procs {
            // Skip kernel threads (no executable path, empty cmdline on linux)
            if process.exe().is_none() && process.cmd().is_empty() {
                continue;
            }

            // Filter by current user unless --all
            if !show_all {
                if let Some(ref cur_user) = current_uid {
                    let proc_user = process.user_id()
                        .and_then(|uid| users.get_user_by_id(uid))
                        .map(|u| u.name().to_string());
                    if proc_user.as_ref() != Some(cur_user) {
                        continue;
                    }
                }
            }

            let pid = process.pid().as_u32();
            let name = process.name().to_string();

            // Build a truncated cmdline label
            let cmdline = process.cmd().iter()
                .map(|a| a.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            let label = if cmdline.is_empty() {
                name.clone()
            } else {
                let truncated = if cmdline.len() > 60 {
                    format!("{}…", &cmdline[..59])
                } else {
                    cmdline
                };
                format!("{name} {truncated}")
            };

            let status_str = match process.status() {
                ProcessStatus::Run => "running",
                ProcessStatus::Sleep | ProcessStatus::Idle => "sleeping",
                ProcessStatus::Zombie => "zombie",
                ProcessStatus::Stop => "stopped",
                _ => "unknown",
            };

            let item = Item::new(slot, pid.to_string(), label)
                .with_status(status_str);
            items.push(item);
            slot += 1;
        }

        Ok(items)
    }

    fn preview_cmd(&self, item: &Item) -> Option<String> {
        Some(format!("lsof -p {}", item.raw))
    }
}
