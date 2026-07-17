mod common;

use noteit::context::resolve;
use noteit::store::Store;
use noteit::store::contexts::Kind;

#[test]
fn repo_with_commits_resolves_to_a_repo_context() {
    let store = Store::open_in_memory().unwrap();
    let dir = common::repo_with_commits(1);
    let r = resolve(&store, dir.path()).unwrap();
    assert_eq!(r.context.kind, Kind::Repo);
    assert!(r.context.key.starts_with("urn:noteit:v1:"));
    assert_eq!(r.subpath, ".");
}

#[test]
fn subdirectory_of_a_repo_records_its_subpath() {
    let store = Store::open_in_memory().unwrap();
    let dir = common::repo_with_commits(1);
    let sub = dir.path().join("src");
    std::fs::create_dir_all(&sub).unwrap();

    let root = resolve(&store, dir.path()).unwrap();
    let inner = resolve(&store, &sub).unwrap();

    // Same context, different subpath -- this is the whole point.
    assert_eq!(root.context.id, inner.context.id);
    assert_eq!(inner.subpath, "src");
}

#[test]
fn plain_directory_resolves_to_a_path_context() {
    let store = Store::open_in_memory().unwrap();
    let dir = common::plain_dir();
    let r = resolve(&store, dir.path()).unwrap();
    assert_eq!(r.context.kind, Kind::Path);
    assert_eq!(r.subpath, ".");
}

#[test]
fn zero_commit_repo_resolves_to_a_path_context() {
    // Falls back to path-binding and adopts later once a root commit exists.
    let store = Store::open_in_memory().unwrap();
    let dir = common::empty_repo();
    let r = resolve(&store, dir.path()).unwrap();
    assert_eq!(r.context.kind, Kind::Path);
}

#[test]
fn shallow_clone_warns_only_once() {
    let store = Store::open_in_memory().unwrap();
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

    let first = resolve(&store, &clone_path).unwrap();
    assert!(first.warning.is_some());

    let second = resolve(&store, &clone_path).unwrap();
    assert!(second.warning.is_none());
}

#[test]
fn display_name_defaults_to_the_directory_basename() {
    let store = Store::open_in_memory().unwrap();
    let dir = common::plain_dir();
    let expected = dir
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let r = resolve(&store, dir.path()).unwrap();
    assert_eq!(r.context.display_name, expected);
}
