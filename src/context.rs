use std::path::Path;

use crate::repoid::{self, RepoIdError};
use crate::store::contexts::{Context, Kind};
use crate::store::{Store, StoreError};

#[derive(Debug, thiserror::Error)]
pub enum ContextError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug)]
pub struct Resolved {
    pub context: Context,
    /// Path of `cwd` relative to `context.root_path`; "." at the root.
    pub subpath: String,
    /// User-facing warning, printed once per run (currently: shallow).
    pub warning: Option<String>,
}

/// The display name for a context: the directory basename.
///
/// Display-only -- it never keys anything, so a wrong default can never
/// split or lose notes. `noteit project rename` overrides it.
pub fn display_name_for(root: &Path) -> String {
    root.file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| root.to_string_lossy().to_string())
}

fn rel_subpath(root: &Path, cwd: &Path) -> String {
    match cwd.strip_prefix(root) {
        Ok(p) if p.as_os_str().is_empty() => ".".to_string(),
        Ok(p) => p.to_string_lossy().replace('\\', "/"),
        Err(_) => ".".to_string(),
    }
}

/// Resolve `cwd` to the context its notes belong to.
///
/// The ladder, per spec: a usable repo id wins; every failure mode falls
/// back to path-binding rather than erroring, because a note tool must
/// never lose a note.
pub fn resolve(store: &Store, cwd: &Path) -> Result<Resolved, ContextError> {
    let cwd = cwd.canonicalize().unwrap_or_else(|_| cwd.to_path_buf());
    let mut warning = None;
    let mut shallow = false;

    match repoid::project_id(&cwd) {
        Ok(id) => {
            let root = repoid::repo_root(&cwd).unwrap_or_else(|_| cwd.clone());
            let root = root.canonicalize().unwrap_or(root);
            let name = display_name_for(&root);
            let context = store.upsert_context(
                Kind::Repo,
                id.as_str(),
                &name,
                &root.to_string_lossy(),
            )?;
            let subpath = rel_subpath(&root, &cwd);
            return Ok(Resolved { context, subpath, warning });
        }
        Err(RepoIdError::Shallow) => {
            shallow = true;
        }
        Err(RepoIdError::NotARepo) | Err(RepoIdError::NoCommits) | Err(RepoIdError::NoHead) => {}
        Err(RepoIdError::Other(msg)) => {
            // Never crash on capture -- degrade to path binding and say why.
            warning = Some(format!("git error, binding to path instead: {msg}"));
        }
    }

    let name = display_name_for(&cwd);
    let key = cwd.to_string_lossy().to_string();
    let context = store.upsert_context(Kind::Path, &key, &name, &key)?;

    if shallow && store.claim_shallow_warning(context.id)? {
        warning = Some(
            "shallow clone: notes bind to this path until you run `git fetch --unshallow`"
                .to_string(),
        );
    }

    Ok(Resolved { context, subpath: ".".to_string(), warning })
}

#[derive(Debug)]
pub struct AdoptionReport {
    pub notes_moved: usize,
    pub paths_folded: usize,
    pub project: String,
}

/// Fold any path contexts at or under this repo's root into it.
///
/// ALL path contexts adopt -- there is no permanent path context. Any
/// directory can become a repo: NoCommits gains a commit, Shallow gains
/// history, and a NotARepo dir gains a `git init`. The submodule guard is
/// the one exclusion.
pub fn adopt_if_needed(
    store: &mut Store,
    resolved: &Resolved,
) -> Result<Option<AdoptionReport>, ContextError> {
    if resolved.context.kind != Kind::Repo {
        return Ok(None);
    }
    let root = resolved.context.root_path.clone();
    let candidates = store.path_contexts_under(&root)?;
    if candidates.is_empty() {
        return Ok(None);
    }

    // Submodule guard: a nested dir whose repository ROOT differs from this
    // repo's root belongs to a different repository and must not be
    // swallowed by the parent. Comparing roots (not ids) matters because
    // `project_id` fails for shallow clones and zero-commit repos -- both
    // are still separate repositories that must not lose their notes.
    let adopting_root = std::path::Path::new(&root);
    let adopting_root_canon =
        adopting_root.canonicalize().unwrap_or_else(|_| adopting_root.to_path_buf());
    let mut adoptable = Vec::new();
    for c in candidates {
        let p = std::path::Path::new(&c.root_path);
        match repoid::repo_root(p) {
            Ok(r) => {
                let r_canon = r.canonicalize().unwrap_or(r);
                if r_canon != adopting_root_canon {
                    continue; // different repository, whatever its state
                }
                adoptable.push(c);
            }
            // NotARepo / Shallow / NoCommits / NoHead all genuinely mean "no
            // different repo root was established here" -- adoptable.
            Err(RepoIdError::NotARepo)
            | Err(RepoIdError::Shallow)
            | Err(RepoIdError::NoCommits)
            | Err(RepoIdError::NoHead) => adoptable.push(c),
            // A real git error, not a repo-shape answer -- be conservative
            // and leave it for a later run rather than risk folding a
            // nested repo's notes into the parent.
            Err(RepoIdError::Other(_)) => continue,
        }
    }
    if adoptable.is_empty() {
        return Ok(None);
    }

    let paths_folded = adoptable.len();
    let notes_moved = store.adopt(&adoptable, resolved.context.id, &root)?;
    // adoptable is non-empty here, so paths_folded > 0 -- always report.

    Ok(Some(AdoptionReport {
        notes_moved,
        paths_folded,
        project: resolved.context.display_name.clone(),
    }))
}
