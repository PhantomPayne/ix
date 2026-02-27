use ix_core::item::Category;
use std::collections::HashMap;

/// Colour theme for status indicators.
///
/// Colours are stored as ANSI 256-colour codes (`u8`) — the same index space
/// that both `crossterm` and `ratatui` natively understand, so a single value
/// feeds both terminal print output and the TUI picker without conversion.
///
/// # Environment variable — `IX_COLORS`
///
/// Set `IX_COLORS` to override any or all defaults.  The format mirrors
/// `LS_COLORS`: colon-separated `key=code` pairs, where `code` is an ANSI-256
/// colour index (0–255).
///
/// ```text
/// IX_COLORS="M=33:A=32:D=31:R=36:T=35:running=32:zombie=31:paused=33"
/// ```
#[derive(Debug, Clone)]
pub struct StatusTheme {
    pub string_map: HashMap<String, u8>,
    pub category_map: HashMap<Category, u8>,
    pub dim: u8,
}

impl Default for StatusTheme {
    fn default() -> Self {
        let mut string_map = HashMap::new();
        string_map.insert("M".to_string(), 3); // ANSI yellow
        string_map.insert("A".to_string(), 2); // ANSI green
        string_map.insert("D".to_string(), 1); // ANSI red
        string_map.insert("R".to_string(), 6); // ANSI cyan
        string_map.insert("T".to_string(), 5); // ANSI magenta
        string_map.insert("??".to_string(), 8); // bright black / dark grey
        string_map.insert("!!".to_string(), 8);
        string_map.insert("*".to_string(), 10); // bright green
        string_map.insert("running".to_string(), 2);
        string_map.insert("sleeping".to_string(), 8);
        string_map.insert("idle".to_string(), 8);
        string_map.insert("zombie".to_string(), 1);
        string_map.insert("stopped".to_string(), 8);
        string_map.insert("exited".to_string(), 8);
        string_map.insert("paused".to_string(), 3);

        let mut category_map = HashMap::new();
        category_map.insert(Category::Positive, 2);
        category_map.insert(Category::Negative, 1);
        category_map.insert(Category::Warning, 3);
        category_map.insert(Category::Neutral, 8);
        category_map.insert(Category::Unknown, 8);

        Self {
            string_map,
            category_map,
            dim: 8,
        }
    }
}

impl StatusTheme {
    /// Build a theme by overlaying any `IX_COLORS` env-var overrides on top of
    /// the compiled-in defaults.
    pub fn from_env() -> Self {
        let mut theme = Self::default();
        if let Ok(raw) = std::env::var("IX_COLORS") {
            theme.apply_str(&raw);
        }
        theme
    }

    /// Apply a colon-separated `key=ansi_code` string (same format as
    /// `LS_COLORS`) to this theme.
    pub fn apply_str(&mut self, s: &str) {
        for entry in s.split(':') {
            let entry = entry.trim();
            if entry.is_empty() {
                continue;
            }
            if let Some((key, val)) = entry.split_once('=') {
                if let Ok(code) = val.trim().parse::<u8>() {
                    self.apply_key(key.trim(), code);
                } else {
                    eprintln!(
                        "ix: warning: invalid color code in IX_COLORS for key '{}'",
                        key.trim()
                    );
                }
            } else {
                eprintln!("ix: warning: invalid entry in IX_COLORS: '{}'", entry);
            }
        }
    }

    fn apply_key(&mut self, key: &str, code: u8) {
        if key == "dim" {
            self.dim = code;
        } else {
            // Apply known aliases for backward compatibility
            let key = match key {
                "modified" => "M",
                "added" => "A",
                "deleted" => "D",
                "renamed" => "R",
                "typechange" => "T",
                "untracked" => "??",
                "ignored" => "!!",
                "staged" => "*",
                k => k,
            };
            self.string_map.insert(key.to_string(), code);
        }
    }

    /// Return the ANSI-256 colour index for a given status string, or `None`
    /// if the status is empty (no colour should be applied).
    pub fn color_for(&self, status: &str, category: Category) -> Option<u8> {
        if status.is_empty() {
            return None;
        }
        if let Some(&code) = self.string_map.get(status) {
            return Some(code);
        }
        if let Some(&code) = self.category_map.get(&category) {
            return Some(code);
        }
        Some(self.dim)
    }

    // ── crossterm helpers (used by display.rs) ────────────────────────────

    /// Wrap `text` in crossterm ANSI colour escape codes lazily to avoid String allocs.
    pub fn colorize<'a>(
        &'a self,
        status: &'a str,
        category: Category,
        text: &'a str,
    ) -> Styled<'a> {
        Styled {
            theme: self,
            status,
            category,
            text,
        }
    }

    /// Wrap `text` in crossterm bold escape codes lazily.
    pub fn bold(text: &str) -> Attributed<'_> {
        use crossterm::style::Attribute;
        Attributed {
            attr: Attribute::Bold,
            text,
        }
    }

    /// Wrap `text` in crossterm dim escape codes lazily.
    pub fn dimmed(text: &str) -> Attributed<'_> {
        use crossterm::style::Attribute;
        Attributed {
            attr: Attribute::Dim,
            text,
        }
    }

    /// Wrap `text` in crossterm italic escape codes lazily.
    pub fn italic(text: &str) -> Attributed<'_> {
        use crossterm::style::Attribute;
        Attributed {
            attr: Attribute::Italic,
            text,
        }
    }

    // ── ratatui helpers (used by picker.rs) ──────────────────────────────

    /// Return a `ratatui` [`Style`] for the given status string and category.
    pub fn ratatui_style(&self, status: &str, category: Category) -> ratatui::style::Style {
        use ratatui::style::{Color, Style};
        match self.color_for(status, category) {
            Some(code) => Style::default().fg(Color::Indexed(code)),
            None => Style::default(),
        }
    }
}

pub struct Styled<'a> {
    theme: &'a StatusTheme,
    status: &'a str,
    category: Category,
    text: &'a str,
}

impl<'a> std::fmt::Display for Styled<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use crossterm::style::{Color, ResetColor, SetForegroundColor};
        match self.theme.color_for(self.status, self.category) {
            Some(code) => {
                write!(f, "{}", SetForegroundColor(Color::AnsiValue(code)))?;
                f.write_str(self.text)?;
                write!(f, "{}", ResetColor)
            }
            None => f.write_str(self.text),
        }
    }
}

pub struct Attributed<'a> {
    attr: crossterm::style::Attribute,
    text: &'a str,
}

impl<'a> std::fmt::Display for Attributed<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use crossterm::style::{Attribute, SetAttribute};
        let cancel = match self.attr {
            Attribute::Bold => Attribute::NormalIntensity,
            Attribute::Dim => Attribute::NormalIntensity,
            Attribute::Italic => Attribute::NoItalic,
            _ => Attribute::Reset,
        };
        write!(f, "{}", SetAttribute(self.attr))?;
        f.write_str(self.text)?;
        write!(f, "{}", SetAttribute(cancel))
    }
}
