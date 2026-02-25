#[cfg(any(test, feature = "test_utils"))]
pub use mock::*;

#[cfg(any(test, feature = "test_utils"))]
pub mod mock {
    use crate::provider::Context;
    use std::path::{Path, PathBuf};

    pub fn ctx_default() -> Context {
        Context::new(PathBuf::from("."))
    }

    pub fn ctx_path(path: &Path) -> Context {
        Context::new(path.to_path_buf())
    }
}
