use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn ix_cmd(dir: &Path, session: &str) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_ix"));
    cmd.current_dir(dir);
    // Use a per-test session ID so parallel tests don't stomp on each other.
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

#[test]
fn test_cli_gs_shows_untracked() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("foo.rs"), "").unwrap();

    let out = ix_cmd(td.path(), "gs-untracked")
        .arg("gs")
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();

    assert!(stdout.contains("[1]"));
    assert!(stdout.contains("foo.rs"));
}

#[test]
fn test_cli_resolves_slot() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("foo.rs"), "").unwrap();

    // first run ix gs to populate the index
    ix_cmd(td.path(), "resolve-slot")
        .arg("gs")
        .output()
        .unwrap();

    // then resolve slot 1
    let out = ix_cmd(td.path(), "resolve-slot").arg("1").output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap().trim().to_string();

    assert!(stdout.ends_with("foo.rs"));
}

#[test]
fn test_cli_resolves_range() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("a.rs"), "").unwrap();
    std::fs::write(td.path().join("b.rs"), "").unwrap();
    std::fs::write(td.path().join("c.rs"), "").unwrap();

    ix_cmd(td.path(), "resolve-range")
        .arg("gs")
        .output()
        .unwrap();

    let out = ix_cmd(td.path(), "resolve-range")
        .arg("1-3")
        .output()
        .unwrap();
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

    ix_cmd(td.path(), "resolve-comma")
        .arg("gs")
        .output()
        .unwrap();

    let out = ix_cmd(td.path(), "resolve-comma")
        .arg("1,3")
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();

    assert!(stdout.contains("a.rs"));
    assert!(!stdout.contains("b.rs"));
    assert!(stdout.contains("c.rs"));
}

#[test]
fn test_cli_index_stable_between_runs() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("foo.rs"), "").unwrap();

    ix_cmd(td.path(), "stable").arg("gs").output().unwrap();

    // resolve twice without refreshing — slot must be stable
    let out1 = ix_cmd(td.path(), "stable").arg("1").output().unwrap();
    let out2 = ix_cmd(td.path(), "stable").arg("1").output().unwrap();

    assert_eq!(out1.stdout, out2.stdout);
}

#[test]
fn test_cli_autodetects_git_repo() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("foo.rs"), "").unwrap();

    // plain `ix` with no subcommand should behave like `ix gs` in a git repo
    let out = ix_cmd(td.path(), "autodetect").output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();

    assert!(stdout.contains("foo.rs"));
}

#[test]
fn test_cli_filter_narrows_results() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("foo.rs"), "").unwrap();
    std::fs::write(td.path().join("bar.txt"), "").unwrap();

    let out = ix_cmd(td.path(), "filter-narrows")
        .arg("gs")
        .arg("-f")
        .arg("*.rs")
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();

    assert!(stdout.contains("foo.rs"), "foo.rs should match filter");
    assert!(!stdout.contains("bar.txt"), "bar.txt should be filtered out");
}

#[test]
fn test_cli_filter_renumbers_slots() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("a.rs"), "").unwrap();
    std::fs::write(td.path().join("b.txt"), "").unwrap();
    std::fs::write(td.path().join("c.rs"), "").unwrap();

    let out = ix_cmd(td.path(), "filter-renumbers")
        .arg("gs")
        .arg("-f")
        .arg("*.rs")
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();

    // Slots should be sequential (1, 2) without the gap from b.txt
    assert!(stdout.contains("[1]"));
    assert!(stdout.contains("[2]"));
    assert!(!stdout.contains("[3]"), "slot 3 should not exist");

    assert!(stdout.contains("a.rs"));
    assert!(stdout.contains("c.rs"));
}

#[test]
fn test_cli_filter_then_resolve() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("foo.rs"), "").unwrap();
    std::fs::write(td.path().join("bar.txt"), "").unwrap();
    std::fs::write(td.path().join("baz.rs"), "").unwrap();

    // Run gs with filter
    let raw_out = ix_cmd(td.path(), "filter-resolve")
        .arg("gs")
        .arg("-f")
        .arg("*.rs")
        .output()
        .unwrap();
    let raw_stdout = String::from_utf8(raw_out.stdout).unwrap();
    println!("GS OUTPUT:\n{raw_stdout}");

    // Resolve slot 1 (should be baz.rs), slot 2 (should be foo.rs)
    let out1 = ix_cmd(td.path(), "filter-resolve").arg("1").output().unwrap();
    let stdout1 = String::from_utf8(out1.stdout).unwrap();
    let stderr1 = String::from_utf8(out1.stderr).unwrap();
    println!("STDOUT: {stdout1}\nSTDERR: {stderr1}");
    assert!(stdout1.contains("baz.rs"));

    let out2 = ix_cmd(td.path(), "filter-resolve").arg("2").output().unwrap();
    let stdout2 = String::from_utf8(out2.stdout).unwrap();
    assert!(stdout2.contains("foo.rs"));
}

#[test]
fn test_cli_gl_shows_commits() {
    let (td, repo) = make_repo();
    let sig = repo.signature().unwrap();
    let tree_id = repo.index().unwrap().write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "first commit", &tree, &[])
        .unwrap();

    let out = ix_cmd(td.path(), "gl").arg("gl").output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();

    assert!(stdout.contains("[1]"));
    assert!(stdout.contains("first commit"));
}

#[test]
fn test_cli_fd_finds_files() {
    let (td, _repo) = make_repo();
    std::fs::create_dir(td.path().join("sub")).unwrap();
    std::fs::write(td.path().join("sub/foo.rs"), "").unwrap();

    let out = ix_cmd(td.path(), "fd").arg("fd").output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();

    assert!(stdout.contains("[1]"));
    // Use path separator agnostic check or just the filename part
    assert!(stdout.contains("foo.rs"));
}

#[test]
fn test_cli_fd_respects_gitignore() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join(".gitignore"), "*.log\n").unwrap();
    std::fs::write(td.path().join("server.log"), "error").unwrap();
    std::fs::write(td.path().join("main.rs"), "fn main() {}").unwrap();

    // Default fd respects gitignore
    let out = ix_cmd(td.path(), "fd-ignore").arg("fd").output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();

    assert!(stdout.contains("main.rs"));
    assert!(!stdout.contains("server.log"));

    // fd -A shows everything
    let out_all = ix_cmd(td.path(), "fd-all")
        .arg("fd")
        .arg("-A")
        .output()
        .unwrap();
    let stdout_all = String::from_utf8(out_all.stdout).unwrap();

    assert!(stdout_all.contains("server.log"));
}

#[test]
fn test_cli_passthrough_execution() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("a.rs"), "").unwrap();
    std::fs::write(td.path().join("b.rs"), "").unwrap();
    std::fs::write(td.path().join("c.rs"), "").unwrap();

    ix_cmd(td.path(), "passthrough")
        .arg("gs")
        .output()
        .unwrap();

    // ix 1-3 -- echo
    let out = ix_cmd(td.path(), "passthrough")
        .arg("1-3")
        .arg("--")
        .arg("echo")
        .arg("hello")
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();

    // The output should be the result of `echo hello a.rs b.rs c.rs`
    assert!(stdout.starts_with("hello"));
    assert!(stdout.contains("a.rs"));
    assert!(stdout.contains("b.rs"));
    assert!(stdout.contains("c.rs"));
}

