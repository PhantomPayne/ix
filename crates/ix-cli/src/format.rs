use std::io::Write;

use ix_core::Item;

use crate::args::OutputFormat;

/// Print items to stdout using the given format.
pub fn output(items: &[&Item], format: &OutputFormat) {
    match format {
        OutputFormat::Default => {
            let raws: Vec<&str> = items.iter().map(|i| i.raw.as_str()).collect();
            print!("{}", raws.join(" "));
        }
        OutputFormat::Lines => {
            println!("{}", format_template(items, "{raw}"));
        }
        OutputFormat::Null => {
            format_null(items);
        }
        OutputFormat::Paste => {
            println!("{}", format_paste(items));
        }
        OutputFormat::Json => {
            println!("{}", format_json(items));
        }
        OutputFormat::Template(t) => {
            println!("{}", format_template(items, t));
        }
    }
}

/// Quoted with trailing comma, one per line.
/// Single item: no trailing comma (e.g. `"value"`).
/// Multiple items: every line ends with a comma (e.g. `"a",\n"b",`).
fn format_paste(items: &[&Item]) -> String {
    if items.len() == 1 {
        format!("\"{}\"", items[0].raw.replace('"', "\\\""))
    } else {
        items
            .iter()
            .map(|i| format!("\"{}\",", i.raw.replace('"', "\\\"")))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// NUL-separated output written directly to stdout.
fn format_null(items: &[&Item]) {
    let mut out = std::io::stdout().lock();
    for item in items {
        out.write_all(item.raw.as_bytes()).unwrap();
        out.write_all(&[0u8]).unwrap();
    }
    out.flush().unwrap();
}

/// JSON array of full item objects.
fn format_json(items: &[&Item]) -> String {
    let arr: Vec<serde_json::Value> = items
        .iter()
        .map(|i| {
            serde_json::json!({
                "slot":   i.slot,
                "raw":    i.raw,
                "label":  i.label,
                "status": i.status,
                "group":  i.group,
            })
        })
        .collect();
    serde_json::to_string_pretty(&serde_json::Value::Array(arr)).unwrap()
}

/// Apply a format string to each item, joining results with newlines.
///
/// Available variables: `{raw}`, `{label}`, `{slot}`, `{status}`, `{group}`.
/// Escape literal braces with `{{` and `}}`.
pub fn format_template(items: &[&Item], template: &str) -> String {
    items
        .iter()
        .map(|item| {
            template
                .replace("{{", "\x00LBRACE\x00")
                .replace("}}", "\x00RBRACE\x00")
                .replace("{raw}",    &item.raw)
                .replace("{label}",  &item.label)
                .replace("{slot}",   &item.slot.to_string())
                .replace("{status}", item.status.as_deref().unwrap_or(""))
                .replace("{group}",  item.group.as_deref().unwrap_or(""))
                .replace("\x00LBRACE\x00", "{")
                .replace("\x00RBRACE\x00", "}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}
