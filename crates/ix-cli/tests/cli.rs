use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

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

    // first run ix gs to populate the index
    ix_cmd(td.path()).arg("gs").output().unwrap();

    // then resolve slot 1
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

    // resolve twice without refreshing — slot must be stable
    let out1 = ix_cmd(td.path()).arg("1").output().unwrap();
    let out2 = ix_cmd(td.path()).arg("1").output().unwrap();

    assert_eq!(out1.stdout, out2.stdout);
}

#[test]
fn test_cli_autodetects_git_repo() {
    let (td, _repo) = make_repo();
    std::fs::write(td.path().join("foo.rs"), "").unwrap();

    // plain `ix` with no subcommand should behave like `ix gs` in a git repo
    let out = ix_cmd(td.path()).output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();

    assert!(stdout.contains("foo.rs"));
}
