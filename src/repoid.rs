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

/// The URN namespace. The SHA payload matches the reference implementation's
/// algorithm exactly, but the namespace is ours -- noteit uses its own
/// namespace. The SHA is the cross-tool join key if that is ever wanted.
const URN_PREFIX: &str = "urn:noteit:v1:";

pub fn project_id(dir: &Path) -> Result<RepoId, RepoIdError> {
    let repo = gix::discover(dir).map_err(|_| RepoIdError::NotARepo)?;

    // A shallow clone's history is truncated, so its oldest reachable
    // commit is not the true root. Reject rather than return a wrong id.
    if repo.is_shallow() {
        return Err(RepoIdError::Shallow);
    }

    let head = repo.head_id().map_err(|e| {
        let msg = e.to_string();
        if msg.contains("does not have any commits") {
            RepoIdError::NoCommits
        } else {
            RepoIdError::NoHead
        }
    })?;

    let walk = repo
        .rev_walk([head])
        .all()
        .map_err(|e| RepoIdError::Other(e.to_string()))?;

    let mut best: Option<String> = None;
    for info in walk {
        let info = info.map_err(|e| RepoIdError::Other(e.to_string()))?;
        if info.parent_ids().next().is_none() {
            let hex = info.id().to_string();
            // Lexicographically smallest wins, matching the reference
            // implementation's strcmp selection so multi-root repos agree across tools.
            if best.as_ref().is_none_or(|b| hex < *b) {
                best = Some(hex);
            }
        }
    }

    match best {
        Some(sha) => Ok(RepoId(format!("{URN_PREFIX}{sha}"))),
        None => Err(RepoIdError::NoCommits),
    }
}

pub fn repo_root(dir: &Path) -> Result<PathBuf, RepoIdError> {
    let repo = gix::discover(dir).map_err(|_| RepoIdError::NotARepo)?;
    repo.workdir()
        .map(|p| p.to_path_buf())
        .ok_or(RepoIdError::Other("bare repository has no workdir".into()))
}
