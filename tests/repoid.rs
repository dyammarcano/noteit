mod common;

use noteit::repoid::{RepoIdError, project_id};

#[test]
fn plain_directory_is_not_a_repo() {
    let dir = common::plain_dir();
    let err = project_id(dir.path()).unwrap_err();
    assert!(matches!(err, RepoIdError::NotARepo), "got {err:?}");
}

#[test]
fn single_root_repo_matches_git_oracle() {
    let dir = common::repo_with_commits(3);
    let id = project_id(dir.path()).expect("id");
    let expected = format!("urn:noteit:v1:{}", common::git_root_sha(dir.path()));
    assert_eq!(id.as_str(), expected);
}

#[test]
fn id_is_identical_after_cloning_to_a_different_path() {
    // THE core premise: identity travels with history, not location.
    let origin = common::repo_with_commits(2);
    let dest = tempfile::tempdir().unwrap();
    let clone_path = dest.path().join("elsewhere");

    common::git(
        dest.path(),
        &[
            "clone",
            "-q",
            origin.path().to_str().unwrap(),
            clone_path.to_str().unwrap(),
        ],
    );

    let a = project_id(origin.path()).expect("origin id");
    let b = project_id(&clone_path).expect("clone id");
    assert_eq!(a, b, "clone at a different path must yield the same id");
}

#[test]
fn zero_commit_repo_is_no_commits_not_not_a_repo() {
    // This is the `git init` case -- exactly when you capture the most
    // ideas about a new project. It must be distinguishable from
    // NotARepo, because both path-bind but only this one is expected.
    let dir = common::empty_repo();
    let err = project_id(dir.path()).unwrap_err();
    assert!(matches!(err, RepoIdError::NoCommits), "got {err:?}");
}

#[test]
fn shallow_clone_is_rejected() {
    let origin = common::repo_with_commits(3);
    let dest = tempfile::tempdir().unwrap();
    let clone_path = dest.path().join("shallow");

    // file:// forces a real transport, which `--depth` requires.
    let url = format!(
        "file:///{}",
        origin.path().to_str().unwrap().replace('\\', "/")
    );
    common::git(
        dest.path(),
        &[
            "clone",
            "-q",
            "--depth",
            "1",
            &url,
            clone_path.to_str().unwrap(),
        ],
    );

    let err = project_id(&clone_path).unwrap_err();
    assert!(matches!(err, RepoIdError::Shallow), "got {err:?}");
}

#[test]
fn multi_root_repo_picks_lexicographically_smallest() {
    let dir = common::repo_with_commits(1);
    let first_root = common::git_root_sha(dir.path());

    // An orphan branch creates a second, unrelated root; merging it into
    // the default branch makes both roots reachable from HEAD.
    common::git(dir.path(), &["checkout", "-q", "--orphan", "second"]);
    common::git(dir.path(), &["rm", "-rq", "--cached", "."]);
    // rm --cached untracks but leaves the old files on disk; clean them so
    // switching back to master (which tracks them) doesn't collide.
    common::git(dir.path(), &["clean", "-fdq"]);
    common::commit_file(dir.path(), "other.txt", "other");
    common::git(dir.path(), &["checkout", "-q", "master"]);
    common::git(
        dir.path(),
        &[
            "merge",
            "-q",
            "--allow-unrelated-histories",
            "-m",
            "merge",
            "second",
        ],
    );

    let roots = common::git(dir.path(), &["rev-list", "--max-parents=0", "HEAD"]);
    let mut all: Vec<&str> = roots.lines().map(str::trim).collect();
    assert_eq!(all.len(), 2, "fixture must produce two roots");
    all.sort_unstable();

    let id = project_id(dir.path()).expect("id");
    assert_eq!(id.as_str(), format!("urn:noteit:v1:{}", all[0]));
    assert!(all[0] == first_root || all[1] == first_root);
}

#[test]
fn orphan_branch_yields_a_different_id_by_design() {
    // Documented, accepted behavior -- NOT a bug. The id is HEAD-relative.
    // Notes store their context at capture time and are never recomputed,
    // so they stay where they landed. This test exists so nobody
    // "fixes" this later without reading the spec.
    let dir = common::repo_with_commits(1);
    let on_master = project_id(dir.path()).expect("master id");

    common::git(dir.path(), &["checkout", "-q", "--orphan", "detached"]);
    common::git(dir.path(), &["rm", "-rq", "--cached", "."]);
    common::commit_file(dir.path(), "solo.txt", "solo");

    let on_orphan = project_id(dir.path()).expect("orphan id");
    assert_ne!(on_master, on_orphan);
}
