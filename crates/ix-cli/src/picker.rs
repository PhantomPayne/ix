use std::io;

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};

use ix_core::{Index, Selection};

/// Run the interactive numbers-only TUI picker.
///
/// The user types slot numbers (e.g. `1`, `1-3`, `1,3`) and matching slots
/// highlight in real time. Enter confirms, Esc/Ctrl+C aborts.
///
/// Exit contract:
/// - Returns `Ok(Some(non-empty))` → confirmed; caller prints and exits 0
/// - Returns `Ok(None)` or `Ok(Some(empty))` → aborted; caller exits 1
pub fn run_picker(index: &Index) -> anyhow::Result<Option<Vec<String>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, index);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

struct App<'a> {
    index: &'a Index,
    /// What the user has typed so far (slot selector syntax)
    input: String,
}

impl<'a> App<'a> {
    fn new(index: &'a Index) -> Self {
        Self { index, input: String::new() }
    }

    /// Slot numbers currently highlighted by the input selector.
    fn highlighted_slots(&self) -> Vec<usize> {
        if self.input.is_empty() {
            return vec![];
        }
        // The input may contain commas or spaces as separators
        let args: Vec<&str> = self.input
            .split_whitespace()
            .flat_map(|s| std::iter::once(s))
            .collect();
        if args.is_empty() {
            return vec![];
        }
        match Selection::parse(&args) {
            Ok(sel) => match sel.resolve(self.index) {
                Ok(items) => items.iter().map(|i| i.slot).collect(),
                Err(_) => vec![],
            },
            Err(_) => vec![],
        }
    }

    /// Resolve the current input to raw item values (for confirmation).
    fn confirm(&self) -> Vec<String> {
        let highlighted = self.highlighted_slots();
        highlighted
            .iter()
            .filter_map(|&slot| self.index.items.iter().find(|i| i.slot == slot))
            .map(|i| i.raw.clone())
            .collect()
    }
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    index: &Index,
) -> anyhow::Result<Option<Vec<String>>> {
    let mut app = App::new(index);

    loop {
        terminal.draw(|f| draw(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            match (key.modifiers, key.code) {
                // Abort
                (_, KeyCode::Esc) => return Ok(None),
                (KeyModifiers::CONTROL, KeyCode::Char('c')) => return Ok(None),

                // Confirm
                (_, KeyCode::Enter) => {
                    return Ok(Some(app.confirm()));
                }

                // Edit input
                (_, KeyCode::Backspace) => {
                    app.input.pop();
                }
                (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char(c)) => {
                    app.input.push(c);
                }

                _ => {}
            }
        }
    }
}

fn draw(f: &mut ratatui::Frame, app: &mut App) {
    // Layout: item list on top, input box at bottom (fixed height)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(4)])
        .split(f.size());

    let highlighted = app.highlighted_slots();

    // ── Item list ──────────────────────────────────────────────────────────
    let items: Vec<ListItem> = app.index.items.iter().map(|item| {
        let is_highlighted = highlighted.contains(&item.slot);

        let slot_span = Span::styled(
            format!("[{}]", item.slot),
            if is_highlighted {
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
            } else {
                Style::default().add_modifier(Modifier::BOLD)
            },
        );
        let status_str = item.status.as_deref().unwrap_or("  ");
        let status_span = Span::styled(
            format!(" {status_str}"),
            style_for_status(status_str),
        );
        let label_span = Span::raw(format!("  {}", item.label));

        let line = Line::from(vec![slot_span, status_span, label_span]);

        let style = if is_highlighted {
            Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        ListItem::new(line).style(style)
    }).collect();

    let provider = &app.index.provider;
    let count = app.index.items.len();
    let title = format!(" {provider}   {count} items ");

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title));

    f.render_widget(list, chunks[0]);

    // ── Input box ──────────────────────────────────────────────────────────
    let input_line = format!("> {}_", app.input);
    let help_line = "  Enter: confirm  Esc: cancel";

    let input_widget = Paragraph::new(format!("{input_line}\n{help_line}"))
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::White));

    f.render_widget(input_widget, chunks[1]);
}

fn style_for_status(status: &str) -> Style {
    match status {
        "M" => Style::default().fg(Color::Yellow),
        "A" => Style::default().fg(Color::Green),
        "D" => Style::default().fg(Color::Red),
        "R" | "T" => Style::default().fg(Color::Cyan),
        "??" | "!!" => Style::default().fg(Color::DarkGray),
        "running" => Style::default().fg(Color::Green),
        "zombie" => Style::default().fg(Color::Red),
        _ => Style::default().fg(Color::DarkGray),
    }
}
