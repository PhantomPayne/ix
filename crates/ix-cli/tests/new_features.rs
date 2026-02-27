use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn ix_cmd(dir: &Path, session: &str) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_ix"));
    cmd.current_dir(dir);
    cmd.env("IX_SESSION_ID", session);
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

// ── shell-init ─────────────────────────────────────────────────────────────

#[test]
fn test_shell_init_contains_zsh_integration() {
    let td = tempdir().unwrap();
    let out = ix_cmd(td.path(), "shell-init-zsh")
        .arg("shell-init")
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();

    assert!(stdout.contains("ZSH_VERSION"));
    assert!(stdout.contains("_ix_widget"));
    assert!(stdout.contains("bindkey"));
    assert!(stdout.contains("IX_SESSION_ID"));
}

#[test]
fn test_shell_init_contains_bash_integration() {
    let td = tempdir().unwrap();
    let out = ix_cmd(td.path(), "shell-init-bash")
        .arg("shell-init")
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();

    assert!(stdout.contains("BASH_VERSION"));
    assert!(stdout.contains("_ix_widget"));
    assert!(stdout.contains("READLINE_LINE"));
    assert!(stdout.contains("bind -x"));
}

#[test]
fn test_shell_init_contains_fish_integration() {
    let td = tempdir().unwrap();
    let out = ix_cmd(td.path(), "shell-init-fish")
        .arg("shell-init")
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();

    assert!(stdout.contains("status is-interactive"));
    assert!(stdout.contains("function _ix_widget"));
    assert!(stdout.contains("commandline -i"));
    assert!(stdout.contains("bind \\cx"));
    assert!(stdout.contains("%self"));
}

// ── JSON output ────────────────────────────────────────────────────────────

#[test]
fn test_json_output_is_valid_json() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("test.rs"), "").unwrap();

    ix_cmd(td.path(), "json-output").arg("gs").output().unwrap();

    let out = ix_cmd(td.path(), "json-output")
        .arg("1")
        .arg("--json")
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid JSON: {e}\nstdout: {stdout}"));
    assert!(parsed.is_array());
    assert_eq!(parsed.as_array().unwrap().len(), 1);
    assert!(parsed[0]["raw"].as_str().unwrap().contains("test.rs"));
    assert_eq!(parsed[0]["slot"], 1);
}

// ── diff ───────────────────────────────────────────────────────────────────

#[test]
fn test_diff_detects_new_items() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("a.rs"), "").unwrap();

    // First index
    ix_cmd(td.path(), "diff-test").arg("gs").output().unwrap();

    // Add a new file
    std::fs::write(td.path().join("b.rs"), "").unwrap();

    // Second index (rotates first → prev)
    ix_cmd(td.path(), "diff-test").arg("gs").output().unwrap();

    // Diff should show b.rs as added
    let out = ix_cmd(td.path(), "diff-test").arg("diff").output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();

    assert!(stdout.contains("b.rs"), "diff should show b.rs as added");
}

#[test]
fn test_diff_detects_removed_items() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("x.rs"), "").unwrap();
    std::fs::write(td.path().join("y.rs"), "").unwrap();

    // First index with both files
    ix_cmd(td.path(), "diff-remove").arg("gs").output().unwrap();

    // Remove one file
    std::fs::remove_file(td.path().join("y.rs")).unwrap();

    // Second index
    ix_cmd(td.path(), "diff-remove").arg("gs").output().unwrap();

    // Diff should show y.rs as removed
    let out = ix_cmd(td.path(), "diff-remove")
        .arg("diff")
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();

    assert!(stdout.contains("y.rs"), "diff should show y.rs as removed");
}

// ── do verb ────────────────────────────────────────────────────────────────

#[test]
fn test_do_verb_executes_command() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("target.txt"), "").unwrap();

    ix_cmd(td.path(), "do-test").arg("gs").output().unwrap();

    let out = ix_cmd(td.path(), "do-test")
        .arg("1")
        .arg("do")
        .arg("echo")
        .arg("FOUND:{}")
        .output()
        .unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(out.status.success());
    // The `do` verb prints the expanded command to stderr
    assert!(
        stderr.contains("FOUND:"),
        "stderr should show expanded command"
    );
}

#[test]
fn test_do_verb_aborts_on_failure() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("fail.txt"), "").unwrap();

    ix_cmd(td.path(), "do-fail").arg("gs").output().unwrap();

    let out = ix_cmd(td.path(), "do-fail")
        .arg("1")
        .arg("do")
        .arg("false")
        .output()
        .unwrap();

    assert!(!out.status.success());
}

// ── env provider ───────────────────────────────────────────────────────────

#[test]
fn test_env_subcommand() {
    let td = tempdir().unwrap();

    let out = ix_cmd(td.path(), "env-test").arg("env").output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();

    assert!(stdout.contains("PATH="));
    assert!(out.status.success());
}

// ── ssh provider ───────────────────────────────────────────────────────────

#[test]
fn test_ssh_subcommand_with_empty_config() {
    let td = tempdir().unwrap();
    // No .ssh/config exists => should succeed with empty output
    let out = ix_cmd(td.path(), "ssh-empty")
        .arg("ssh")
        .env("HOME", td.path())
        .output()
        .unwrap();

    assert!(out.status.success());
}

// ── stdin provider and negative selections ─────────────────────────────────

#[test]
fn test_stdin_provider_auto_detect_and_negative_selection() {
    use std::io::Write;
    let td = tempdir().unwrap();

    let mut index_cmd = ix_cmd(td.path(), "stdin-test");
    // Feed stdin to 'stdin' subcommand to build index
    index_cmd.arg("-");
    index_cmd.stdin(std::process::Stdio::piped());

    let mut child = index_cmd.spawn().unwrap();
    {
        let stdin = child.stdin.as_mut().unwrap();
        writeln!(stdin, "apple\nbanana\ncherry\ndate").unwrap();
    }
    let status = child.wait().unwrap();
    assert!(status.success());

    // Resolve slots 1-4 except 2 and 4 (should yield apple cherry)
    let out = ix_cmd(td.path(), "stdin-test")
        .arg("1-4")
        .arg("^2")
        .arg("^4")
        .output()
        .unwrap();

    let stdout = String::from_utf8(out.stdout).unwrap();
    let stdout = stdout.trim();
    assert_eq!(stdout, "apple cherry");
}
