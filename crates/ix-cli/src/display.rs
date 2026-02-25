use crate::theme::StatusTheme;
use indexmap::IndexSet;
use ix_core::{Index, Item};

/// Padded display width for the status column so columns align regardless of
/// which statuses are present.  The longest status string is "sleeping" (8),
/// but "sleeping" is displayed as "sleep  " (7 chars) — keep this at 7.
const STATUS_WIDTH: usize = 7;

/// Print the index to stdout with colorized output, using the provided theme.
///
/// Colours are applied via crossterm escape codes so the same colour indices
/// work consistently across the non-TUI output and the ratatui picker.
pub fn print_index(index: &Index, theme: &StatusTheme) {
    if index.items.is_empty() {
        println!("  {}", StatusTheme::dimmed("(nothing to show)"));
        return;
    }

    let max_width = index.max_slot_width();

    // Collect groups in insertion order, deduplicated.
    let seen_groups: IndexSet<Option<String>> =
        index.items.iter().map(|i| i.group.clone()).collect();

    for group in &seen_groups {
        // Print group header
        if let Some(g) = group {
            println!();
            let dimmed = format!("{}", StatusTheme::dimmed(g));
            println!("  {}", StatusTheme::italic(&dimmed));
        }

        // Print items in this group
        for item in index.items.iter().filter(|i| &i.group == group) {
            print_item(item, max_width, theme);
        }
    }
    println!();
}

fn print_item(item: &Item, max_width: usize, theme: &StatusTheme) {
    let slot_str = format!("[{:>width$}]", item.slot, width = max_width);
    let slot = StatusTheme::bold(&slot_str);
    let (status_str, category) = item
        .status
        .as_ref()
        .map(|s| (s.text.as_str(), s.category))
        .unwrap_or(("", ix_core::item::Category::Unknown));
    let status_pad = pad_status(status_str);
    let status = theme.colorize(status_str, category, &status_pad);
    let label = &item.label;
    println!("  {slot} {status}  {label}");
}

/// Pad a status string to `STATUS_WIDTH` characters so columns align.
fn pad_status(status: &str) -> String {
    format!("{:<width$}", status, width = STATUS_WIDTH)
}
