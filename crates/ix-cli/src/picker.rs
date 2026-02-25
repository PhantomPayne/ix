use std::collections::HashMap;
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
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};

use crate::theme::StatusTheme;
use ix_core::{Index, Item, Selection};

/// Run the interactive TUI picker.
///
/// Returns the selected raw strings, or `None` if the user cancelled.
/// Colours are drawn from `theme` which may be configured via `IX_COLORS`.
pub fn run_picker(index: &Index, theme: &StatusTheme) -> anyhow::Result<Option<Vec<String>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, index, theme);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

pub(crate) struct App<'a> {
    pub(crate) index: &'a Index,
    pub(crate) input: String,
    /// Items currently visible (after filtering by typed slot numbers)
    pub(crate) visible: Vec<usize>, // indices into index.items
    /// The resolved item-indices when `input` is a valid Selection, or None
    pub(crate) resolved: Option<Vec<usize>>,
    /// Items selected via Space toggle
    pub(crate) selected: Vec<usize>, // indices into index.items
    pub(crate) list_state: ListState,
    /// Slot → vec-index map for O(1) lookup (built once, immutable)
    pub(crate) item_positions: HashMap<usize, usize>,
}

impl<'a> App<'a> {
    pub(crate) fn new(index: &'a Index) -> Self {
        let visible = (0..index.items.len()).collect();
        let mut list_state = ListState::default();
        if !index.items.is_empty() {
            list_state.select(Some(0));
        }
        let item_positions: HashMap<usize, usize> = index
            .items
            .iter()
            .enumerate()
            .map(|(vec_idx, item)| (item.slot, vec_idx))
            .collect();
        Self {
            index,
            input: String::new(),
            visible,
            resolved: None,
            selected: Vec::new(),
            list_state,
            item_positions,
        }
    }

    pub(crate) fn items(&self) -> &[Item] {
        &self.index.items
    }

    pub(crate) fn update_visible(&mut self) {
        if self.input.is_empty() {
            self.visible = (0..self.items().len()).collect();
            self.resolved = None;
            return;
        }

        let args: Vec<&str> = self.input.split_whitespace().collect();
        if let Ok(sel) = Selection::parse(&args) {
            if let Ok(items) = sel.resolve(self.index) {
                let indices: Vec<usize> = items
                    .iter()
                    .filter_map(|item| self.item_positions.get(&item.slot).copied())
                    .collect();
                self.visible = indices.clone();
                self.resolved = Some(indices);
            } else {
                self.visible.clear();
                self.resolved = None;
            }
        } else {
            self.visible.clear();
            self.resolved = None;
        }

        // Reset list cursor
        if !self.visible.is_empty() {
            let cur = self.list_state.selected().unwrap_or(0);
            if cur >= self.visible.len() {
                self.list_state.select(Some(0));
            }
        }
    }

    pub(crate) fn move_up(&mut self) {
        if self.visible.is_empty() {
            return;
        }
        let cur = self.list_state.selected().unwrap_or(0);
        if cur > 0 {
            self.list_state.select(Some(cur - 1));
        }
    }

    pub(crate) fn move_down(&mut self) {
        if self.visible.is_empty() {
            return;
        }
        let cur = self.list_state.selected().unwrap_or(0);
        if cur + 1 < self.visible.len() {
            self.list_state.select(Some(cur + 1));
        }
    }

    pub(crate) fn toggle_current(&mut self) {
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

    pub(crate) fn confirm(&self) -> Vec<String> {
        // If there are resolved items from the typed input, return those.
        if let Some(ref indices) = self.resolved {
            return indices
                .iter()
                .map(|&i| self.items()[i].raw.clone())
                .collect();
        }
        // If items were space-toggled, return them in slot order.
        if !self.selected.is_empty() {
            let mut out: Vec<String> = self
                .selected
                .iter()
                .map(|&i| self.items()[i].raw.clone())
                .collect();
            out.sort_by_key(|raw| {
                // find slot for this raw value via position map
                self.items()
                    .iter()
                    .find(|i| &i.raw == raw)
                    .and_then(|i| self.item_positions.get(&i.slot).copied())
                    .unwrap_or(usize::MAX)
            });
            return out;
        }
        // Otherwise, return the cursor-highlighted item.
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
    theme: &StatusTheme,
) -> anyhow::Result<Option<Vec<String>>> {
    let mut app = App::new(index);

    loop {
        terminal.draw(|f| draw(f, &mut app, theme))?;

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

pub(crate) fn draw(f: &mut ratatui::Frame, app: &mut App, theme: &StatusTheme) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(f.size());

    // Input box
    let input_block = Block::default().borders(Borders::ALL).title(" ix ");
    let input_widget = Paragraph::new(app.input.as_str())
        .block(input_block)
        .style(Style::default().fg(Color::White));
    f.render_widget(input_widget, chunks[0]);

    // Position cursor at end of input
    f.set_cursor(chunks[0].x + app.input.len() as u16 + 1, chunks[0].y + 1);

    let max_width = app.index.max_slot_width();

    let items: Vec<ListItem> = app
        .visible
        .iter()
        .map(|&item_idx| {
            let item = &app.index.items[item_idx];
            let is_selected = app.selected.contains(&item_idx);

            let slot_span = Span::styled(
                format!("[{:>width$}]", item.slot, width = max_width),
                Style::default().add_modifier(Modifier::BOLD),
            );
            let (status_str, category) = item
                .status
                .as_ref()
                .map(|s| (s.text.as_str(), s.category))
                .unwrap_or(("", ix_core::item::Category::Unknown));
            let status_span = Span::styled(
                format!(" {status_str}"),
                theme.ratatui_style(status_str, category),
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

            ListItem::new(line).style(Style::default())
        })
        .collect();

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
