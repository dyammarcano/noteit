use std::path::{Path, PathBuf};

/// A stable, location-independent repository identity.
///
/// The payload is the lexicographically smallest parentless (root) commit
/// SHA reachable from HEAD -- the same value `git rev-list --max-parents=0
/// HEAD | sort | head -1` reports.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RepoId(String);

impl RepoId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RepoId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Why a repo id could not be produced.
///
/// Deliberately an enum, not `Option`: the context ladder needs to tell
/// these apart. `Shallow` is user-recoverable and warns; the rest are
/// silent normal states.
#[derive(Debug, thiserror::Error)]
pub enum RepoIdError {
    #[error("not a git repository")]
    NotARepo,
    #[error("shallow clone has no true root commit; run `git fetch --unshallow`")]
    Shallow,
    #[error("repository has no commits yet")]
    NoCommits,
    #[error("HEAD could not be resolved")]
    NoHead,
    #[error("git error: {0}")]
    Other(String),
}

pub fn project_id(dir: &Path) -> Result<RepoId, RepoIdError> {
    let _ = gix::discover(dir).map_err(|_| RepoIdError::NotARepo)?;
    Err(RepoIdError::NoHead)
}

pub fn repo_root(dir: &Path) -> Result<PathBuf, RepoIdError> {
    let repo = gix::discover(dir).map_err(|_| RepoIdError::NotARepo)?;
    repo.workdir()
        .map(|p| p.to_path_buf())
        .ok_or(RepoIdError::Other("bare repository has no workdir".into()))
}
