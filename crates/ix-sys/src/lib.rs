use ix_core::error::Result;
use ix_core::item::{Category, Item};
use ix_core::{Context, Provider};
use sysinfo::{ProcessStatus, System};

#[derive(Default)]
pub struct SysProvider;

impl SysProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Provider for SysProvider {
    fn name(&self) -> &str {
        "ps"
    }

    fn detect(&self, _ctx: &Context) -> bool {
        true // always available
    }

    fn list(&self, ctx: &Context) -> Result<Vec<Item>> {
        let show_all = ctx.has_flag("a") || ctx.has_flag("all");

        let mut sys = System::new_all();
        sys.refresh_all();

        // Resolve the current process UID via sysinfo so we don't rely on
        // environment variables that may be absent or spoofed.
        let my_pid = sysinfo::Pid::from_u32(std::process::id());
        let current_uid = sys.process(my_pid).and_then(|p| p.user_id().cloned());

        let mut items = Vec::new();

        let mut procs: Vec<_> = sys.processes().values().collect();
        // Sort by PID for stable ordering
        procs.sort_unstable_by_key(|p| p.pid());

        for process in procs {
            // Skip kernel threads (no executable path, empty cmdline on linux)
            if process.exe().is_none() && process.cmd().is_empty() {
                continue;
            }

            // Filter by current user unless --all
            if !show_all {
                if let Some(ref cur_uid) = current_uid {
                    if process.user_id() != Some(cur_uid) {
                        continue;
                    }
                }
            }

            let pid = process.pid().as_u32();
            let name = process.name().to_string();

            // Build a truncated cmdline label
            let cmdline = process
                .cmd()
                .iter()
                .map(|a| a.to_string())
                .collect::<Vec<_>>()
                .join(" ");
            let label = if cmdline.is_empty() {
                name.clone()
            } else {
                let truncated = if cmdline.chars().count() > 60 {
                    format!("{}…", cmdline.chars().take(59).collect::<String>())
                } else {
                    cmdline
                };
                format!("{name} {truncated}")
            };

            let (status_str, category) = match process.status() {
                ProcessStatus::Run => ("running", Category::Positive),
                ProcessStatus::Sleep | ProcessStatus::Idle => ("sleeping", Category::Neutral),
                ProcessStatus::Zombie => ("zombie", Category::Negative),
                ProcessStatus::Stop => ("stopped", Category::Warning),
                _ => ("unknown", Category::Unknown),
            };

            let item = Item::new(0, pid.to_string(), label).with_status(status_str, category);
            items.push(item);
        }

        Ok(items)
    }

    fn preview_cmd(&self, item: &Item) -> Option<String> {
        Some(format!("lsof -p {}", item.raw))
    }
}

#[cfg(test)]
use ix_core::test_utils::ctx_default;

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn test_ps_finds_spawned_process() {
        let mut child = Command::new("sleep").arg("30").spawn().unwrap();
        let pid = child.id();

        let items = SysProvider::new().list(&ctx_default()).unwrap();

        assert!(
            items.iter().any(|i| i.raw == pid.to_string()),
            "expected to find pid {pid} in process list"
        );

        child.kill().unwrap();
        child.wait().unwrap();
    }

    #[test]
    fn test_ps_process_disappears_after_kill() {
        let mut child = Command::new("sleep").arg("30").spawn().unwrap();
        let pid = child.id();
        child.kill().unwrap();
        child.wait().unwrap();

        // re-fetch after kill
        let items = SysProvider::new().list(&ctx_default()).unwrap();

        assert!(
            !items.iter().any(|i| i.raw == pid.to_string()),
            "killed process {pid} should not appear in list"
        );
    }

    #[test]
    fn test_ps_item_has_label() {
        let mut child = Command::new("sleep").arg("30").spawn().unwrap();
        let pid = child.id();

        let items = SysProvider::new().list(&ctx_default()).unwrap();
        let item = items.iter().find(|i| i.raw == pid.to_string()).unwrap();

        assert!(item.label.contains("sleep"));

        child.kill().unwrap();
        child.wait().unwrap();
    }

    #[test]
    fn test_ps_raw_is_pid_string() {
        let mut child = Command::new("sleep").arg("30").spawn().unwrap();
        let pid = child.id();

        let items = SysProvider::new().list(&ctx_default()).unwrap();
        let item = items.iter().find(|i| i.raw == pid.to_string()).unwrap();

        // raw must be parseable as a u32 pid for use in kill, lsof etc.
        assert!(item.raw.parse::<u32>().is_ok());

        child.kill().unwrap();
        child.wait().unwrap();
    }
}
