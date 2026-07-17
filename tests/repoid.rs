mod common;

use noteit::repoid::{project_id, RepoIdError};

#[test]
fn plain_directory_is_not_a_repo() {
    let dir = common::plain_dir();
    let err = project_id(dir.path()).unwrap_err();
    assert!(matches!(err, RepoIdError::NotARepo), "got {err:?}");
}
