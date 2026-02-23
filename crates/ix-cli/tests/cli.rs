use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

/// Create an `ix` command pointing at the given directory with a stable IX_SESSION.
///
/// The session is derived from the tempdir name so that all `ix` invocations
/// within the same test share one index (at `/tmp/ix-<session>`) without
/// colliding with other parallel tests.
fn ix_cmd(dir: &Path) -> Command {
    let session = dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("test-session")
        .to_string();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_ix"));
    cmd.current_dir(dir);
    cmd.env("IX_SESSION", session);
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

// ── Provider / display tests ──────────────────────────────────────────────────

#[test]
fn test_cli_gs_shows_untracked() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("foo.rs"), "").unwrap();

    let out = ix_cmd(td.path()).arg("gs").output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();

    assert!(stdout.contains("[1]"), "expected slot [1] in output");
    assert!(stdout.contains("foo.rs"), "expected foo.rs in output");
}

#[test]
fn test_cli_resolves_slot() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("foo.rs"), "").unwrap();

    ix_cmd(td.path()).arg("gs").output().unwrap();

    let out = ix_cmd(td.path()).arg("1").output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();

    // Default output is shell-quoted
    assert!(stdout.contains("foo.rs"), "expected foo.rs in output");
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

    // Resolve twice without refreshing — slot must be stable
    let out1 = ix_cmd(td.path()).arg("1").output().unwrap();
    let out2 = ix_cmd(td.path()).arg("1").output().unwrap();

    assert_eq!(out1.stdout, out2.stdout);
}

#[test]
fn test_cli_autodetects_git_repo() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("foo.rs"), "").unwrap();

    // Plain `ix` with no subcommand should behave like `ix gs` in a git repo
    let out = ix_cmd(td.path()).output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();

    assert!(stdout.contains("foo.rs"));
}

// ── Shell quoting ─────────────────────────────────────────────────────────────

#[test]
fn test_default_output_is_shell_quoted() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("foo.rs"), "").unwrap();

    ix_cmd(td.path()).arg("gs").output().unwrap();

    let out = ix_cmd(td.path()).arg("1").output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap().trim().to_string();

    // Default output wraps value in single quotes
    assert!(stdout.starts_with('\''), "expected single-quoted output, got: {stdout}");
    assert!(stdout.ends_with('\''), "expected single-quoted output, got: {stdout}");
    assert!(stdout.contains("foo.rs"));
}

#[test]
fn test_lines_output_is_unquoted() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("foo.rs"), "").unwrap();

    ix_cmd(td.path()).arg("gs").output().unwrap();

    let out = ix_cmd(td.path()).args(["1", "--lines"]).output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap().trim().to_string();

    assert!(!stdout.starts_with('\''), "lines output should not be quoted: {stdout}");
    assert!(stdout.ends_with("foo.rs"), "expected foo.rs at end of output, got: {stdout}");
}

// ── --pick-input (non-interactive testing) ────────────────────────────────────

#[test]
fn test_pick_input_resolves_slot() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("a.rs"), "").unwrap();
    std::fs::write(td.path().join("b.rs"), "").unwrap();

    ix_cmd(td.path()).arg("gs").output().unwrap();

    let out = ix_cmd(td.path())
        .args(["--pick-input", "1,2"])
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(0), "expected exit 0");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("a.rs"), "expected a.rs in: {stdout}");
    assert!(stdout.contains("b.rs"), "expected b.rs in: {stdout}");
}

#[test]
fn test_pick_input_empty_exits_1() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("foo.rs"), "").unwrap();
    ix_cmd(td.path()).arg("gs").output().unwrap();

    let out = ix_cmd(td.path())
        .args(["--pick-input", ""])
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(1), "expected exit 1 for empty input");
    assert!(out.stdout.is_empty(), "expected no stdout on abort");
}

#[test]
fn test_pick_input_no_index_exits_1() {
    let td = tempdir().unwrap();
    // No `ix gs` run → no index file at the session path

    let out = ix_cmd(td.path())
        .args(["--pick-input", "1"])
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(1), "expected exit 1 when no index");
}

#[test]
fn test_pick_input_range() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("a.rs"), "").unwrap();
    std::fs::write(td.path().join("b.rs"), "").unwrap();
    std::fs::write(td.path().join("c.rs"), "").unwrap();

    ix_cmd(td.path()).arg("gs").output().unwrap();

    let out = ix_cmd(td.path())
        .args(["--pick-input", "1-3"])
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("a.rs"));
    assert!(stdout.contains("b.rs"));
    assert!(stdout.contains("c.rs"));
}

// ── `ix init <shell>` ─────────────────────────────────────────────────────────

#[test]
fn test_init_zsh_prints_snippet() {
    let td = tempdir().unwrap();
    let out = ix_cmd(td.path())
        .args(["init", "zsh"])
        .output()
        .unwrap();

    assert!(out.status.success(), "ix init zsh should exit 0");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("IX_SESSION"), "zsh snippet should export IX_SESSION");
    assert!(stdout.contains("_ix_widget"), "zsh snippet should define _ix_widget");
    assert!(stdout.contains("zle -N"), "zsh snippet should register with zle");
    // Should NOT bind any key by default — any uncommented bindkey line starts at column 0
    let has_active_bindkey = stdout.lines().any(|l| l.starts_with("bindkey "));
    assert!(!has_active_bindkey, "zsh snippet should not have uncommented bindkey by default");
}

#[test]
fn test_init_bash_prints_snippet() {
    let td = tempdir().unwrap();
    let out = ix_cmd(td.path())
        .args(["init", "bash"])
        .output()
        .unwrap();

    assert!(out.status.success(), "ix init bash should exit 0");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("IX_SESSION"), "bash snippet should export IX_SESSION");
    assert!(stdout.contains("_ix_widget"), "bash snippet should define _ix_widget");
    assert!(stdout.contains("READLINE_LINE"), "bash snippet should update READLINE_LINE");
    // Should NOT bind any key by default — any active bind -x line starts at column 0
    let has_active_bind = stdout.lines().any(|l| l.starts_with("bind -x "));
    assert!(!has_active_bind, "bash snippet should not have uncommented bind -x by default");
}

#[test]
fn test_init_fish_prints_snippet() {
    let td = tempdir().unwrap();
    let out = ix_cmd(td.path())
        .args(["init", "fish"])
        .output()
        .unwrap();

    assert!(out.status.success(), "ix init fish should exit 0");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("IX_SESSION"), "fish snippet should export IX_SESSION");
    assert!(stdout.contains("_ix_widget"), "fish snippet should define _ix_widget");
}

#[test]
fn test_init_unknown_shell_exits_nonzero() {
    let td = tempdir().unwrap();
    let out = ix_cmd(td.path())
        .args(["init", "powershell"])
        .output()
        .unwrap();

    assert!(!out.status.success(), "ix init <unknown> should exit non-zero");
}

// ── --help / --version ────────────────────────────────────────────────────────

#[test]
fn test_help_exits_0() {
    let td = tempdir().unwrap();
    let out = ix_cmd(td.path()).arg("--help").output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("ix —"), "help should include tool description");
    assert!(stdout.contains("PROVIDERS:"), "help should list providers");
    assert!(stdout.contains("SLOT SYNTAX:"), "help should explain slot syntax");
    assert!(stdout.contains("ix init"), "help should mention shell integration");
}

#[test]
fn test_version_exits_0() {
    let td = tempdir().unwrap();
    let out = ix_cmd(td.path()).arg("--version").output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("ix "), "version output should start with 'ix '");
    assert!(stdout.contains("0.1.0"), "version output should contain version");
}
