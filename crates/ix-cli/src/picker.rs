use std::io;

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};

use ix_core::{Index, Item, Selection};

/// Run the interactive TUI picker. Returns the selected raw strings, or None if cancelled.
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
    input: String,
    /// Items currently visible (after filtering)
    visible: Vec<usize>, // indices into index.items
    /// Items selected via Space toggle
    selected: Vec<usize>, // indices into index.items
    list_state: ListState,
}

impl<'a> App<'a> {
    fn new(index: &'a Index) -> Self {
        let visible = (0..index.items.len()).collect();
        let mut list_state = ListState::default();
        if !index.items.is_empty() {
            list_state.select(Some(0));
        }
        Self {
            index,
            input: String::new(),
            visible,
            selected: Vec::new(),
            list_state,
        }
    }

    fn items(&self) -> &[Item] {
        &self.index.items
    }

    /// Determine if the current input looks like a slot selector.
    fn is_selector_mode(&self) -> bool {
        if self.input.is_empty() {
            return false;
        }
        let args: Vec<&str> = self.input.split_whitespace().collect();
        Selection::parse(&args).is_ok()
    }

    /// Items highlighted by the current selector input.
    fn selector_highlighted(&self) -> Vec<usize> {
        if !self.is_selector_mode() {
            return vec![];
        }
        let args: Vec<&str> = self.input.split_whitespace().collect();
        if let Ok(sel) = Selection::parse(&args) {
            if let Ok(items) = sel.resolve(self.index) {
                return items.iter()
                    .filter_map(|item| self.index.items.iter().position(|i| i.slot == item.slot))
                    .collect();
            }
        }
        vec![]
    }

    fn update_visible(&mut self) {
        if self.input.is_empty() || self.is_selector_mode() {
            self.visible = (0..self.items().len()).collect();
            return;
        }
        // Fuzzy filter
        let matcher = SkimMatcherV2::default();
        let mut scored: Vec<(usize, i64)> = self.items().iter().enumerate()
            .filter_map(|(i, item)| {
                matcher.fuzzy_match(&item.label, &self.input).map(|score| (i, score))
            })
            .collect();
        scored.sort_by(|a, b| b.1.cmp(&a.1)); // highest score first
        self.visible = scored.into_iter().map(|(i, _)| i).collect();

        // Reset list cursor
        if !self.visible.is_empty() {
            let cur = self.list_state.selected().unwrap_or(0);
            if cur >= self.visible.len() {
                self.list_state.select(Some(0));
            }
        }
    }

    fn move_up(&mut self) {
        if self.visible.is_empty() { return; }
        let cur = self.list_state.selected().unwrap_or(0);
        if cur > 0 {
            self.list_state.select(Some(cur - 1));
        }
    }

    fn move_down(&mut self) {
        if self.visible.is_empty() { return; }
        let cur = self.list_state.selected().unwrap_or(0);
        if cur + 1 < self.visible.len() {
            self.list_state.select(Some(cur + 1));
        }
    }

    fn toggle_current(&mut self) {
        if let Some(cur) = self.list_state.selected() {
            if cur < self.visible.len() {
                let item_idx = self.visible[cur];
                if let Some(pos) = self.selected.iter().position(|&i| i == item_idx) {
                    self.selected.remove(pos);
                } else {
                    self.selected.push(item_idx);
                }
            }
        }
    }

    /// Confirm: if in selector mode, use those items; if items are toggle-selected use those;
    /// otherwise use the cursor item.
    fn confirm(&self) -> Vec<String> {
        if self.is_selector_mode() {
            let highlighted = self.selector_highlighted();
            return highlighted.iter()
                .map(|&i| self.items()[i].raw.clone())
                .collect();
        }
        if !self.selected.is_empty() {
            let mut out: Vec<String> = self.selected.iter()
                .map(|&i| self.items()[i].raw.clone())
                .collect();
            // Keep original slot order
            out.sort_by_key(|raw| {
                self.items().iter().find(|i| &i.raw == raw).map(|i| i.slot).unwrap_or(0)
            });
            return out;
        }
        if let Some(cur) = self.list_state.selected() {
            if cur < self.visible.len() {
                let item_idx = self.visible[cur];
                return vec![self.items()[item_idx].raw.clone()];
            }
        }
        vec![]
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
                // Exit / cancel
                (_, KeyCode::Esc) => return Ok(None),
                (KeyModifiers::CONTROL, KeyCode::Char('c')) => return Ok(None),

                // Confirm
                (_, KeyCode::Enter) => {
                    let out = app.confirm();
                    return Ok(Some(out));
                }

                // Navigation
                (_, KeyCode::Up) | (KeyModifiers::NONE, KeyCode::Char('k')) => app.move_up(),
                (_, KeyCode::Down) | (KeyModifiers::NONE, KeyCode::Char('j')) => app.move_down(),

                // Toggle selection
                (_, KeyCode::Char(' ')) => app.toggle_current(),

                // Input
                (_, KeyCode::Backspace) => {
                    app.input.pop();
                    app.update_visible();
                }
                (_, KeyCode::Char(c)) => {
                    app.input.push(c);
                    app.update_visible();
                }

                _ => {}
            }
        }
    }
}

fn draw(f: &mut ratatui::Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(f.size());

    // Input box
    let mode = if app.is_selector_mode() { "number" } else { "fuzzy" };
    let input_block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" ix — {mode} mode "));
    let input_widget = Paragraph::new(app.input.as_str())
        .block(input_block)
        .style(Style::default().fg(Color::White));
    f.render_widget(input_widget, chunks[0]);

    // Position cursor at end of input
    f.set_cursor(
        chunks[0].x + app.input.len() as u16 + 1,
        chunks[0].y + 1,
    );

    // Item list
    let selector_highlighted = app.selector_highlighted();

    let items: Vec<ListItem> = app.visible.iter().map(|&item_idx| {
        let item = &app.index.items[item_idx];
        let is_selected = app.selected.contains(&item_idx);
        let is_highlighted = selector_highlighted.contains(&item_idx);

        let slot_span = Span::styled(
            format!("[{}]", item.slot),
            Style::default().add_modifier(Modifier::BOLD),
        );
        let status_str = item.status.as_deref().unwrap_or("  ");
        let status_span = Span::styled(
            format!(" {status_str}"),
            style_for_status(status_str),
        );
        let label_span = Span::raw(format!("  {}", item.label));

        let check = if is_selected { "●" } else { " " };
        let check_span = Span::styled(
            format!("{check} "),
            if is_selected {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            },
        );

        let line = Line::from(vec![check_span, slot_span, status_span, label_span]);

        let style = if is_highlighted {
            Style::default().bg(Color::DarkGray)
        } else {
            Style::default()
        };

        ListItem::new(line).style(style)
    }).collect();

    let help = " ↑↓/jk navigate  Space select  Enter confirm  Esc cancel ";
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(help))
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, chunks[1], &mut app.list_state);
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
