use colored::Colorize;
use ix_core::{Index, Item};

/// Print the index to stdout with colorized output.
pub fn print_index(index: &Index) {
    if index.items.is_empty() {
        println!("{}", "  (nothing to show)".dimmed());
        return;
    }

    // Collect groups in the order they first appear
    let mut seen_groups: Vec<Option<String>> = Vec::new();
    for item in &index.items {
        if !seen_groups.contains(&item.group) {
            seen_groups.push(item.group.clone());
        }
    }

    let max_slot = index.items.iter().map(|i| i.slot).max().unwrap_or(1);
    let slot_width = max_slot.to_string().len();

    for group in &seen_groups {
        // Print group header
        if let Some(g) = group {
            println!();
            println!("  {}", g.italic().dimmed());
        }

        // Print items in this group
        for item in index.items.iter().filter(|i| &i.group == group) {
            print_item(item, slot_width);
        }
    }
    println!();
}

fn print_item(item: &Item, slot_width: usize) {
    let slot = format!("[{:>width$}]", item.slot, width = slot_width).bold();
    let status = format_status(item.status.as_deref().unwrap_or(""));
    let label = &item.label;
    println!("  {slot} {status}  {label}");
}

fn format_status(status: &str) -> String {
    match status {
        "M" => "M".yellow().to_string(),
        "A" => "A".green().to_string(),
        "D" => "D".red().to_string(),
        "R" => "R".cyan().to_string(),
        "T" => "T".magenta().to_string(),
        "??" => "??".dimmed().to_string(),
        "!!" => "!!".dimmed().to_string(),
        "*" => "*".bright_green().to_string(),
        "running" => "running".green().to_string(),
        "sleeping" | "idle" => "sleep  ".dimmed().to_string(),
        "zombie" => "zombie ".red().to_string(),
        "stopped" => "stopped".dimmed().to_string(),
        "exited" => "exited ".dimmed().to_string(),
        "paused" => "paused ".yellow().to_string(),
        other => other.to_string(),
    }
}

/// Print a staleness warning to stderr.
pub fn print_stale_warning(age_secs: u64) {
    let minutes = age_secs / 60;
    eprintln!(
        "{}",
        format!("  warning: index is {minutes}m old — run `ix` to refresh").yellow()
    );
}
