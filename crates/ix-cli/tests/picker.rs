use ix_core::item::{Category, Item};
use ix_core::Index;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

// Re-use the ix crate's internal picker module
// (App and draw are pub(crate), accessible from integration tests within the same crate)
#[path = "../src/picker.rs"]
#[allow(dead_code)]
mod picker;
#[path = "../src/theme.rs"]
#[allow(dead_code)]
mod theme;

use picker::{draw, App};
use theme::StatusTheme;

fn make_index() -> Index {
    let items = vec![
        Item::new(1, "/path/foo.rs", "foo.rs")
            .with_status("M", Category::Warning)
            .with_group("unstaged"),
        Item::new(2, "/path/bar.rs", "bar.rs")
            .with_status("??", Category::Neutral)
            .with_group("untracked"),
        Item::new(3, "/path/baz.rs", "baz.rs")
            .with_status("A", Category::Positive)
            .with_group("staged"),
    ];
    Index::new("git-status", items)
}

// ── App Logic Tests ────────────────────────────────────────────────────────

#[test]
fn test_app_initializes_with_all_items_visible() {
    let index = make_index();
    let app = App::new(&index);
    assert_eq!(app.items().len(), 3);
}

#[test]
fn test_app_move_down_and_up() {
    let index = make_index();
    let mut app = App::new(&index);

    // Starts at 0
    assert_eq!(app.list_state.selected(), Some(0));

    app.move_down();
    assert_eq!(app.list_state.selected(), Some(1));

    app.move_down();
    assert_eq!(app.list_state.selected(), Some(2));

    // Can't go past the end
    app.move_down();
    assert_eq!(app.list_state.selected(), Some(2));

    app.move_up();
    assert_eq!(app.list_state.selected(), Some(1));

    app.move_up();
    assert_eq!(app.list_state.selected(), Some(0));

    // Can't go past the start
    app.move_up();
    assert_eq!(app.list_state.selected(), Some(0));
}

#[test]
fn test_app_toggle_selection() {
    let index = make_index();
    let mut app = App::new(&index);

    // Toggle first item
    app.toggle_current();
    assert_eq!(app.selected, vec![0]);

    // Move down and toggle second item
    app.move_down();
    app.toggle_current();
    assert_eq!(app.selected, vec![0, 1]);

    // Toggle first item again (deselect)
    app.move_up();
    app.toggle_current();
    assert_eq!(app.selected, vec![1]);
}

#[test]
fn test_app_confirm_returns_cursor_item_when_nothing_selected() {
    let index = make_index();
    let app = App::new(&index);

    let result = app.confirm();
    assert_eq!(result, vec!["/path/foo.rs"]);
}

#[test]
fn test_app_confirm_returns_toggled_items() {
    let index = make_index();
    let mut app = App::new(&index);

    app.toggle_current(); // select item 0
    app.move_down();
    app.move_down();
    app.toggle_current(); // select item 2

    let result = app.confirm();
    assert!(result.contains(&"/path/foo.rs".to_string()));
    assert!(result.contains(&"/path/baz.rs".to_string()));
    assert!(!result.contains(&"/path/bar.rs".to_string()));
}

#[test]
fn test_app_input_filters_by_slot_number() {
    let index = make_index();
    let mut app = App::new(&index);

    // Type "1" to filter to just slot 1
    app.input.push('1');
    app.update_visible();
    assert_eq!(app.visible.len(), 1);

    let result = app.confirm();
    assert_eq!(result, vec!["/path/foo.rs"]);
}

#[test]
fn test_app_input_range_resolves_multiple() {
    let index = make_index();
    let mut app = App::new(&index);

    app.input = "1-3".to_string();
    app.update_visible();
    assert_eq!(app.visible.len(), 3);

    let result = app.confirm();
    assert_eq!(
        result,
        vec!["/path/foo.rs", "/path/bar.rs", "/path/baz.rs"]
    );
}

#[test]
fn test_app_input_comma_selection() {
    let index = make_index();
    let mut app = App::new(&index);

    app.input = "1,3".to_string();
    app.update_visible();
    assert_eq!(app.visible.len(), 2);

    let result = app.confirm();
    assert_eq!(result, vec!["/path/foo.rs", "/path/baz.rs"]);
}

#[test]
fn test_app_backspace_restores_visibility() {
    let index = make_index();
    let mut app = App::new(&index);

    app.input = "1".to_string();
    app.update_visible();
    assert_eq!(app.visible.len(), 1);

    // Backspace to clear input
    app.input.pop();
    app.update_visible();
    assert_eq!(app.visible.len(), 3);
}

#[test]
fn test_app_invalid_input_clears_visible() {
    let index = make_index();
    let mut app = App::new(&index);

    app.input = "99".to_string();
    app.update_visible();
    assert!(app.visible.is_empty());
}

#[test]
fn test_app_empty_index() {
    let index = Index::new("empty", vec![]);
    let mut app = App::new(&index);

    assert_eq!(app.list_state.selected(), None);

    // Operations should not panic on empty index
    app.move_down();
    app.move_up();
    app.toggle_current();
    assert!(app.confirm().is_empty());
}

// ── Render Tests (TestBackend) ─────────────────────────────────────────────

#[test]
fn test_draw_renders_without_panic() {
    let index = make_index();
    let mut app = App::new(&index);
    let theme = StatusTheme::default();

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|f| draw(f, &mut app, &theme)).unwrap();

    let buf = terminal.backend().buffer().clone();
    let content = buf_to_string(&buf);

    // Should contain slot numbers and filenames
    assert!(content.contains("[1]"));
    assert!(content.contains("[2]"));
    assert!(content.contains("[3]"));
    assert!(content.contains("foo.rs"));
    assert!(content.contains("bar.rs"));
    assert!(content.contains("baz.rs"));
}

#[test]
fn test_draw_shows_input_text() {
    let index = make_index();
    let mut app = App::new(&index);
    app.input = "1-2".to_string();
    app.update_visible();
    let theme = StatusTheme::default();

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|f| draw(f, &mut app, &theme)).unwrap();

    let buf = terminal.backend().buffer().clone();
    let content = buf_to_string(&buf);

    assert!(content.contains("1-2"));
}

#[test]
fn test_draw_shows_status_text() {
    let index = make_index();
    let mut app = App::new(&index);
    let theme = StatusTheme::default();

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|f| draw(f, &mut app, &theme)).unwrap();

    let buf = terminal.backend().buffer().clone();
    let content = buf_to_string(&buf);

    assert!(content.contains("M"));
    assert!(content.contains("??"));
    assert!(content.contains("A"));
}

#[test]
fn test_draw_help_text() {
    let index = make_index();
    let mut app = App::new(&index);
    let theme = StatusTheme::default();

    let backend = TestBackend::new(100, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|f| draw(f, &mut app, &theme)).unwrap();

    let buf = terminal.backend().buffer().clone();
    let content = buf_to_string(&buf);

    assert!(content.contains("navigate"));
    assert!(content.contains("Esc cancel"));
}

/// Helper to convert a ratatui Buffer into a single string for assertions.
fn buf_to_string(buf: &ratatui::buffer::Buffer) -> String {
    let area = buf.area;
    let mut s = String::new();
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            s.push(buf.get(x, y).symbol().chars().next().unwrap_or(' '));
        }
        s.push('\n');
    }
    s
}
