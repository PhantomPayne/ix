mod branch;
mod log;
mod stash;
mod status;
mod worktree;

pub use branch::GitBranchProvider;
pub use log::GitLogProvider;
pub use stash::GitStashProvider;
pub use status::GitStatusProvider;
pub use worktree::GitWorktreeProvider;

use git2::Repository;
use ix_core::{
    error::{IxError, Result},
    Context,
};

/// Open the git repository at or above `ctx.cwd`, mapping the error to
/// [`IxError::Provider`].
pub(crate) fn open_repo(ctx: &Context) -> Result<Repository> {
    Repository::discover(&ctx.cwd).map_err(|e| IxError::Provider(format!("git: {e}")))
}

/// Return `true` if a git repository can be found at or above `ctx.cwd`.
pub(crate) fn detect_git(ctx: &Context) -> bool {
    Repository::discover(&ctx.cwd).is_ok()
}

#[cfg(test)]
pub(crate) mod test_helpers {
    use git2::Repository;
    use tempfile::TempDir;

    pub fn make_repo() -> (TempDir, Repository) {
        let td = tempfile::tempdir().unwrap();
        let repo = Repository::init(td.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "test").unwrap();
        config.set_str("user.email", "test@test.com").unwrap();
        (td, repo)
    }

    pub fn make_initial_commit(repo: &Repository) {
        let sig = repo.signature().unwrap();
        let tree_id = repo.index().unwrap().write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .unwrap();
    }
}
