use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use tempfile::tempdir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn ix_cmd(dir: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_ix"));
    cmd.current_dir(dir);
    cmd
}

fn make_repo() -> (tempfile::TempDir, git2::Repository) {
    let td = tempdir().unwrap();
    let repo = git2::Repository::init(td.path()).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "test").unwrap();
    config.set_str("user.email", "test@test.com").unwrap();
    (td, repo)
}

/// Run the ix binary with explicit piped stdin.
fn ix_with_stdin(dir: &Path, stdin_input: &str, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_ix"));
    cmd.current_dir(dir)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(stdin_input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

/// Build an IxContext for the given path (used to compute the index location).
fn ctx(path: &Path) -> ix_core::Context {
    ix_core::Context::new(path.to_path_buf())
}

// ---------------------------------------------------------------------------
// Existing tests (unchanged)
// ---------------------------------------------------------------------------

#[test]
fn test_cli_gs_shows_untracked() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("foo.rs"), "").unwrap();

    let out = ix_cmd(td.path()).arg("gs").output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();

    assert!(stdout.contains("[1]"));
    assert!(stdout.contains("foo.rs"));
}

#[test]
fn test_cli_resolves_slot() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("foo.rs"), "").unwrap();

    ix_cmd(td.path()).arg("gs").output().unwrap();

    let out = ix_cmd(td.path()).arg("1").output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap().trim().to_string();

    assert!(stdout.ends_with("foo.rs"));
}

#[test]
fn test_cli_resolves_range() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("a.rs"), "").unwrap();
    std::fs::write(td.path().join("b.rs"), "").unwrap();
    std::fs::write(td.path().join("c.rs"), "").unwrap();

    ix_cmd(td.path()).arg("gs").output().unwrap();

    let out = ix_cmd(td.path()).arg("1-3").output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();

    assert!(stdout.contains("a.rs"));
    assert!(stdout.contains("b.rs"));
    assert!(stdout.contains("c.rs"));
}

#[test]
fn test_cli_resolves_comma_separated() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("a.rs"), "").unwrap();
    std::fs::write(td.path().join("b.rs"), "").unwrap();
    std::fs::write(td.path().join("c.rs"), "").unwrap();

    ix_cmd(td.path()).arg("gs").output().unwrap();

    let out = ix_cmd(td.path()).arg("1,3").output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();

    assert!(stdout.contains("a.rs"));
    assert!(!stdout.contains("b.rs"));
    assert!(stdout.contains("c.rs"));
}

#[test]
fn test_cli_index_stable_between_runs() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("foo.rs"), "").unwrap();

    ix_cmd(td.path()).arg("gs").output().unwrap();

    let out1 = ix_cmd(td.path()).arg("1").output().unwrap();
    let out2 = ix_cmd(td.path()).arg("1").output().unwrap();

    assert_eq!(out1.stdout, out2.stdout);
}

#[test]
fn test_cli_autodetects_git_repo() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("foo.rs"), "").unwrap();

    let out = ix_cmd(td.path()).output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();

    assert!(stdout.contains("foo.rs"));
}

// ---------------------------------------------------------------------------
// Stdin mode tests
// ---------------------------------------------------------------------------

#[test]
fn test_stdin_assigns_slots() {
    let td = tempdir().unwrap();
    let input = "alpha\nbeta\ngamma\n";

    let out = ix_with_stdin(td.path(), input, &[]);
    let stdout = String::from_utf8(out.stdout).unwrap();

    assert!(stdout.contains("[1]") && stdout.contains("alpha"));
    assert!(stdout.contains("[2]") && stdout.contains("beta"));
    assert!(stdout.contains("[3]") && stdout.contains("gamma"));
}

#[test]
fn test_stdin_selects_single_slot() {
    let td = tempdir().unwrap();
    let input = "alpha\nbeta\ngamma\n";

    let out = ix_with_stdin(td.path(), input, &["2"]);
    let stdout = String::from_utf8(out.stdout).unwrap().trim().to_string();

    assert_eq!(stdout, "beta");
}

#[test]
fn test_stdin_selects_range() {
    let td = tempdir().unwrap();
    let input = "alpha\nbeta\ngamma\ndelta\n";

    let out = ix_with_stdin(td.path(), input, &["1-3"]);
    let stdout = String::from_utf8(out.stdout).unwrap();

    assert!(stdout.contains("alpha"));
    assert!(stdout.contains("beta"));
    assert!(stdout.contains("gamma"));
    assert!(!stdout.contains("delta"));
}

#[test]
fn test_stdin_selects_comma_separated() {
    let td = tempdir().unwrap();
    let input = "alpha\nbeta\ngamma\n";

    let out = ix_with_stdin(td.path(), input, &["1,3"]);
    let stdout = String::from_utf8(out.stdout).unwrap();

    assert!(stdout.contains("alpha"));
    assert!(!stdout.contains("beta"));
    assert!(stdout.contains("gamma"));
}

#[test]
fn test_stdin_skips_empty_lines() {
    let td = tempdir().unwrap();
    let input = "alpha\n\nbeta\n\ngamma\n";

    let out = ix_with_stdin(td.path(), input, &[]);
    let stdout = String::from_utf8(out.stdout).unwrap();

    assert!(stdout.contains("[1]") && stdout.contains("alpha"));
    assert!(stdout.contains("[2]") && stdout.contains("beta"));
    assert!(stdout.contains("[3]") && stdout.contains("gamma"));
    assert!(!stdout.contains("[4]"));
}

#[test]
fn test_stdin_strips_trailing_whitespace() {
    let td = tempdir().unwrap();
    let input = "alpha   \nbeta\t\n";

    let out = ix_with_stdin(td.path(), input, &["1"]);
    let stdout = String::from_utf8(out.stdout).unwrap().trim().to_string();

    assert_eq!(stdout, "alpha");
}

#[test]
fn test_stdin_whole_line_is_raw_value() {
    let td = tempdir().unwrap();
    // simulate git branch output — documented limitation
    let input = "* main\n  feature-x\n";

    let out = ix_with_stdin(td.path(), input, &["1"]);
    let stdout = String::from_utf8(out.stdout).unwrap().trim().to_string();

    // whole line including "* " prefix — this is expected behavior
    assert_eq!(stdout, "* main");
}

#[test]
fn test_stdin_saves_index_in_display_mode() {
    let td = tempdir().unwrap();

    ix_with_stdin(td.path(), "alpha\nbeta\n", &[]);

    let index = ix_core::Index::read(&ix_core::Index::index_path(&ctx(td.path()))).unwrap();
    assert_eq!(index.items.len(), 2);
    assert_eq!(index.provider, "stdin");
}

#[test]
fn test_stdin_resolves_from_saved_index() {
    let td = tempdir().unwrap();

    // display mode saves the index
    ix_with_stdin(td.path(), "alpha\nbeta\ngamma\n", &[]);

    // resolve without re-piping — must read from the saved index
    let out = ix_cmd(td.path()).arg("2").output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap().trim().to_string();

    assert_eq!(stdout, "beta");
}

// ---------------------------------------------------------------------------
// Save-behavior tests
// ---------------------------------------------------------------------------

#[test]
fn test_provider_mode_saves_index() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("foo.rs"), "").unwrap();

    ix_cmd(td.path()).arg("gs").output().unwrap();

    let index_path = td.path().join(".git/ix-index");
    assert!(index_path.exists());
}

#[test]
fn test_stdin_inline_select_does_not_save_index() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("foo.rs"), "").unwrap();

    // populate index with provider
    ix_cmd(td.path()).arg("gs").output().unwrap();
    let before = ix_core::Index::read(&ix_core::Index::index_path(&ctx(td.path()))).unwrap();

    // inline select — should NOT overwrite the index
    ix_with_stdin(td.path(), "alpha\nbeta\n", &["1"]);

    let after = ix_core::Index::read(&ix_core::Index::index_path(&ctx(td.path()))).unwrap();
    assert_eq!(before.provider, after.provider);
    assert_eq!(before.items.len(), after.items.len());
}

#[test]
fn test_no_save_preserves_existing_index() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("foo.rs"), "").unwrap();

    ix_cmd(td.path()).arg("gs").output().unwrap();
    let before = ix_core::Index::read(&ix_core::Index::index_path(&ctx(td.path()))).unwrap();

    // add another file then run gs --no-save
    std::fs::write(td.path().join("bar.rs"), "").unwrap();
    ix_cmd(td.path()).args(&["gs", "--no-save"]).output().unwrap();

    let after = ix_core::Index::read(&ix_core::Index::index_path(&ctx(td.path()))).unwrap();
    assert_eq!(before.items.len(), after.items.len());
}

#[test]
fn test_no_save_still_displays_current_state() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("foo.rs"), "").unwrap();
    std::fs::write(td.path().join("bar.rs"), "").unwrap();

    let out = ix_cmd(td.path()).args(&["gs", "--no-save"]).output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();

    assert!(stdout.contains("foo.rs"));
    assert!(stdout.contains("bar.rs"));
}

// ---------------------------------------------------------------------------
// Output format tests
// ---------------------------------------------------------------------------

#[test]
fn test_format_default_space_separated() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("a.rs"), "").unwrap();
    std::fs::write(td.path().join("b.rs"), "").unwrap();
    ix_cmd(td.path()).arg("gs").output().unwrap();

    let out = ix_cmd(td.path()).args(&["1", "2"]).output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap().trim().to_string();

    assert!(stdout.contains("a.rs") && stdout.contains("b.rs"));
    assert!(!stdout.contains('"'));
    assert_eq!(stdout.lines().count(), 1);
}

#[test]
fn test_format_lines() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("a.rs"), "").unwrap();
    std::fs::write(td.path().join("b.rs"), "").unwrap();
    std::fs::write(td.path().join("c.rs"), "").unwrap();
    ix_cmd(td.path()).arg("gs").output().unwrap();

    let out = ix_cmd(td.path()).args(&["1-3", "--lines"]).output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();

    assert_eq!(lines.len(), 3);
    assert!(lines[0].ends_with("a.rs"));
    assert!(lines[1].ends_with("b.rs"));
    assert!(lines[2].ends_with("c.rs"));
}

#[test]
fn test_format_null_separated() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("a.rs"), "").unwrap();
    std::fs::write(td.path().join("b.rs"), "").unwrap();
    ix_cmd(td.path()).arg("gs").output().unwrap();

    let out = ix_cmd(td.path()).args(&["1-2", "--null"]).output().unwrap();
    let parts: Vec<&[u8]> = out.stdout.split(|&b| b == 0).collect();

    // two values + trailing NUL → 3 parts, last empty
    assert_eq!(parts.len(), 3);
    assert!(String::from_utf8_lossy(parts[0]).ends_with("a.rs"));
    assert!(String::from_utf8_lossy(parts[1]).ends_with("b.rs"));
    assert!(parts[2].is_empty());
}

#[test]
fn test_format_paste_single() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("main.rs"), "").unwrap();
    ix_cmd(td.path()).arg("gs").output().unwrap();

    let out = ix_cmd(td.path()).args(&["1", "--paste"]).output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap().trim().to_string();

    assert!(stdout.starts_with('"') && stdout.ends_with('"'));
    assert!(stdout.contains("main.rs"));
    assert!(!stdout.contains(','));
}

#[test]
fn test_format_paste_multiple() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("a.rs"), "").unwrap();
    std::fs::write(td.path().join("b.rs"), "").unwrap();
    std::fs::write(td.path().join("c.rs"), "").unwrap();
    ix_cmd(td.path()).arg("gs").output().unwrap();

    let out = ix_cmd(td.path()).args(&["1-3", "--paste"]).output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap().trim().to_string();

    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 3);
    for line in &lines {
        assert!(line.starts_with('"'));
        assert!(line.ends_with(','));
    }
}

#[test]
fn test_format_paste_escapes_inner_quotes() {
    let td = tempdir().unwrap();
    // use stdin to create a value with a double-quote in it
    ix_with_stdin(td.path(), "file\"quoted\".rs\n", &[]);

    let out = ix_cmd(td.path()).args(&["1", "--paste"]).output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap().trim().to_string();

    assert!(stdout.contains("\\\""));
}

#[test]
fn test_format_json_structure() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("main.rs"), "").unwrap();
    ix_cmd(td.path()).arg("gs").output().unwrap();

    let out = ix_cmd(td.path()).args(&["1", "--json"]).output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let arr = parsed.as_array().unwrap();

    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["slot"], 1);
    assert!(arr[0]["raw"].as_str().unwrap().ends_with("main.rs"));
    assert!(arr[0].get("status").is_some());
    assert!(arr[0].get("group").is_some());
}

#[test]
fn test_format_json_is_valid_json() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("a.rs"), "").unwrap();
    std::fs::write(td.path().join("b.rs"), "").unwrap();
    ix_cmd(td.path()).arg("gs").output().unwrap();

    let out = ix_cmd(td.path()).args(&["1-2", "--json"]).output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed.as_array().unwrap().len(), 2);
}

// ---------------------------------------------------------------------------
// Passthrough mode tests
// ---------------------------------------------------------------------------

#[test]
fn test_passthrough_expands_at_sigil() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("foo.rs"), "fn main() {}\n").unwrap();
    ix_cmd(td.path()).arg("gs").output().unwrap();

    // ix wc -l @1 should expand @1 to the filepath
    let out = ix_cmd(td.path())
        .args(&["wc", "-l", "@1"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();

    assert!(stdout.contains("foo.rs"));
}

#[test]
fn test_passthrough_bare_number_not_expanded() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("foo.rs"), "").unwrap();
    ix_cmd(td.path()).arg("gs").output().unwrap();

    // 755 has no @ so it must not be treated as a slot
    let out = ix_cmd(td.path())
        .args(&["chmod", "755", "@1"])
        .output()
        .unwrap();

    assert!(out.status.success());
    let meta = std::fs::metadata(td.path().join("foo.rs")).unwrap();
    let mode = std::os::unix::fs::PermissionsExt::mode(&meta.permissions());
    assert_eq!(mode & 0o777, 0o755);
}

#[test]
fn test_passthrough_double_dash_no_expansion() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("foo.rs"), "").unwrap();
    ix_cmd(td.path()).arg("gs").output().unwrap();

    let out = ix_cmd(td.path())
        .args(&["echo", "--", "@1"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap().trim().to_string();

    assert_eq!(stdout, "@1");
}

#[test]
fn test_passthrough_expands_range() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("a.rs"), "").unwrap();
    std::fs::write(td.path().join("b.rs"), "").unwrap();
    std::fs::write(td.path().join("c.rs"), "").unwrap();
    ix_cmd(td.path()).arg("gs").output().unwrap();

    let out = ix_cmd(td.path())
        .args(&["echo", "@1-3"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();

    assert!(stdout.contains("a.rs"));
    assert!(stdout.contains("b.rs"));
    assert!(stdout.contains("c.rs"));
}

#[test]
fn test_passthrough_mixed_literal_and_slot() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("foo.rs"), "").unwrap();
    ix_cmd(td.path()).arg("gs").output().unwrap();

    let out = ix_cmd(td.path())
        .args(&["echo", "before", "@1", "after"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();

    assert!(stdout.contains("before"));
    assert!(stdout.contains("foo.rs"));
    assert!(stdout.contains("after"));
}

#[test]
fn test_passthrough_comma_range() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("a.rs"), "").unwrap();
    std::fs::write(td.path().join("b.rs"), "").unwrap();
    std::fs::write(td.path().join("c.rs"), "").unwrap();
    ix_cmd(td.path()).arg("gs").output().unwrap();

    let out = ix_cmd(td.path())
        .args(&["echo", "@1,3"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();

    assert!(stdout.contains("a.rs"));
    assert!(!stdout.contains("b.rs"));
    assert!(stdout.contains("c.rs"));
}

// ---------------------------------------------------------------------------
// Stdin + format combined
// ---------------------------------------------------------------------------

#[test]
fn test_stdin_display_then_paste_format() {
    let td = tempdir().unwrap();
    ix_with_stdin(td.path(), "src/main.rs\nsrc/lib.rs\ntests/foo.rs\n", &[]);

    let out = ix_cmd(td.path()).args(&["1-3", "--paste"]).output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap().trim().to_string();

    assert_eq!(
        stdout,
        "\"src/main.rs\",\n\"src/lib.rs\",\n\"tests/foo.rs\","
    );
}

#[test]
fn test_stdin_inline_select_with_lines() {
    let td = tempdir().unwrap();

    let out = ix_with_stdin(td.path(), "alpha\nbeta\ngamma\n", &["1,3", "--lines"]);
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();

    assert_eq!(lines, vec!["alpha", "gamma"]);
}

#[test]
fn test_stdin_inline_select_with_null() {
    let td = tempdir().unwrap();

    let out = ix_with_stdin(td.path(), "alpha\nbeta\ngamma\n", &["1-2", "--null"]);
    let parts: Vec<&[u8]> = out.stdout.split(|&b| b == 0).collect();

    assert_eq!(parts.len(), 3); // two values + trailing NUL
    assert_eq!(parts[0], b"alpha");
    assert_eq!(parts[1], b"beta");
}

// ---------------------------------------------------------------------------
// --template tests
// ---------------------------------------------------------------------------

#[test]
fn test_template_raw_variable() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("main.rs"), "").unwrap();
    ix_cmd(td.path()).arg("gs").output().unwrap();

    let out = ix_cmd(td.path())
        .args(&["1", "--template", "{raw}"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap().trim().to_string();

    assert!(stdout.ends_with("main.rs"));
}

#[test]
fn test_template_slot_variable() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("main.rs"), "").unwrap();
    ix_cmd(td.path()).arg("gs").output().unwrap();

    let out = ix_cmd(td.path())
        .args(&["1", "--template", "{slot}: {raw}"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap().trim().to_string();

    assert!(stdout.starts_with("1: "));
    assert!(stdout.ends_with("main.rs"));
}

#[test]
fn test_template_label_variable() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("main.rs"), "").unwrap();
    ix_cmd(td.path()).arg("gs").output().unwrap();

    let out = ix_cmd(td.path())
        .args(&["1", "--template", "{label}"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap().trim().to_string();

    // label is just the filename, not the full path
    assert_eq!(stdout, "main.rs");
}

#[test]
fn test_template_status_variable() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("main.rs"), "").unwrap();
    ix_cmd(td.path()).arg("gs").output().unwrap();

    let out = ix_cmd(td.path())
        .args(&["1", "--template", "{label} [{status}]"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap().trim().to_string();

    assert!(stdout.starts_with("main.rs ["));
    assert!(stdout.ends_with(']'));
}

#[test]
fn test_template_one_line_per_item() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("a.rs"), "").unwrap();
    std::fs::write(td.path().join("b.rs"), "").unwrap();
    std::fs::write(td.path().join("c.rs"), "").unwrap();
    ix_cmd(td.path()).arg("gs").output().unwrap();

    let out = ix_cmd(td.path())
        .args(&["1-3", "--template", "{slot}:{label}"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();

    assert_eq!(lines.len(), 3);
    assert!(lines[0].starts_with("1:"));
    assert!(lines[1].starts_with("2:"));
    assert!(lines[2].starts_with("3:"));
}

#[test]
fn test_template_literal_braces_escaped() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("main.rs"), "").unwrap();
    ix_cmd(td.path()).arg("gs").output().unwrap();

    let out = ix_cmd(td.path())
        .args(&["1", "--template", "{{literal}} {raw}"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap().trim().to_string();

    assert!(stdout.starts_with("{literal} "));
    assert!(stdout.ends_with("main.rs"));
}

#[test]
fn test_template_paste_equivalence() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("a.rs"), "").unwrap();
    std::fs::write(td.path().join("b.rs"), "").unwrap();
    ix_cmd(td.path()).arg("gs").output().unwrap();

    let paste_out = ix_cmd(td.path()).args(&["1-2", "--paste"]).output().unwrap();
    let template_out = ix_cmd(td.path())
        .args(&["1-2", "--template", r#""{raw}","#])
        .output()
        .unwrap();

    // --paste and --template '"{raw}",' must produce identical output for multiple items
    assert_eq!(paste_out.stdout, template_out.stdout);
}

#[test]
fn test_template_lines_equivalence() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("a.rs"), "").unwrap();
    std::fs::write(td.path().join("b.rs"), "").unwrap();
    ix_cmd(td.path()).arg("gs").output().unwrap();

    let lines_out = ix_cmd(td.path()).args(&["1-2", "--lines"]).output().unwrap();
    let template_out = ix_cmd(td.path())
        .args(&["1-2", "--template", "{raw}"])
        .output()
        .unwrap();

    // --lines and --template '{raw}' must produce identical output
    assert_eq!(lines_out.stdout, template_out.stdout);
}

#[test]
fn test_template_short_flag() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("main.rs"), "").unwrap();
    ix_cmd(td.path()).arg("gs").output().unwrap();

    let long_out = ix_cmd(td.path())
        .args(&["1", "--template", "{raw}"])
        .output()
        .unwrap();
    let short_out = ix_cmd(td.path())
        .args(&["1", "-t", "{raw}"])
        .output()
        .unwrap();

    assert_eq!(long_out.stdout, short_out.stdout);
}

#[test]
fn test_template_stdin_items() {
    let td = tempdir().unwrap();
    ix_with_stdin(td.path(), "alpha\nbeta\ngamma\n", &[]);

    let out = ix_cmd(td.path())
        .args(&["1-3", "-t", "{slot}. {raw}"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();

    assert_eq!(lines, vec!["1. alpha", "2. beta", "3. gamma"]);
}

// ---------------------------------------------------------------------------
// ix do tests
// ---------------------------------------------------------------------------

#[test]
fn test_do_runs_command_per_slot() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("a.rs"), "line1\n").unwrap();
    std::fs::write(td.path().join("b.rs"), "line1\nline2\n").unwrap();
    ix_cmd(td.path()).arg("gs").output().unwrap();

    let out = ix_cmd(td.path())
        .args(&["do", "@1-2", "touch"])
        .output()
        .unwrap();

    assert!(out.status.success());
}

#[test]
fn test_do_appends_value_as_last_arg() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("foo.rs"), "hello\n").unwrap();
    ix_cmd(td.path()).arg("gs").output().unwrap();

    // wc -l <file> — value appended at end
    let out = ix_cmd(td.path())
        .args(&["do", "@1", "wc", "-l"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();

    assert!(stdout.contains("foo.rs"));
    assert!(out.status.success());
}

#[test]
fn test_do_placeholder_inserts_value_in_position() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("foo.rs"), "").unwrap();
    ix_cmd(td.path()).arg("gs").output().unwrap();

    // cp {} {}.bak — {} replaced with value in both positions
    let out = ix_cmd(td.path())
        .args(&["do", "@1", "cp", "{}", "{}.bak"])
        .output()
        .unwrap();

    assert!(out.status.success());
    assert!(td.path().join("foo.rs.bak").exists());
}

#[test]
fn test_do_runs_in_order() {
    let td = tempdir().unwrap();
    let log = td.path().join("order.log");

    // use stdin to create ordered slots
    ix_with_stdin(td.path(), "first\nsecond\nthird\n", &[]);

    // append each value to a log file via shell substitution in the command string
    ix_cmd(td.path())
        .args(&[
            "do",
            "@1-3",
            "sh",
            "-c",
            &format!("echo {{}} >> {}", log.display()),
        ])
        .output()
        .unwrap();

    let contents = std::fs::read_to_string(&log).unwrap();
    let lines: Vec<&str> = contents.lines().collect();
    assert_eq!(lines, vec!["first", "second", "third"]);
}

#[test]
fn test_do_stops_on_first_failure() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("a.rs"), "").unwrap();
    std::fs::write(td.path().join("b.rs"), "").unwrap();
    std::fs::write(td.path().join("c.rs"), "").unwrap();
    ix_cmd(td.path()).arg("gs").output().unwrap();

    // false always fails — should stop after first item
    let out = ix_cmd(td.path())
        .args(&["do", "@1-3", "false"])
        .output()
        .unwrap();

    assert!(!out.status.success());
}

#[test]
fn test_do_comma_selection() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("a.rs"), "").unwrap();
    std::fs::write(td.path().join("b.rs"), "").unwrap();
    std::fs::write(td.path().join("c.rs"), "").unwrap();
    ix_cmd(td.path()).arg("gs").output().unwrap();

    let before_b = std::fs::metadata(td.path().join("b.rs"))
        .unwrap()
        .modified()
        .unwrap();

    // only touch slots 1 and 3, not 2
    ix_cmd(td.path())
        .args(&["do", "@1,3", "touch"])
        .output()
        .unwrap();

    let after_b = std::fs::metadata(td.path().join("b.rs"))
        .unwrap()
        .modified()
        .unwrap();

    // b.rs mtime should be unchanged
    assert_eq!(before_b, after_b);
}
