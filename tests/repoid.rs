mod common;

use noteit::repoid::{project_id, RepoIdError};

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
