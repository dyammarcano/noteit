mod common;

use noteit::context::{adopt_if_needed, resolve};
use noteit::store::contexts::Kind;
use noteit::store::Store;

#[test]
fn path_notes_fold_into_the_repo_context_preserving_subpaths() {
    let mut store = Store::open_in_memory().unwrap();
    let dir = common::empty_repo(); // zero commits -> path context
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();

    // Capture at the root and in src BEFORE the first commit.
    let at_root = resolve(&store, dir.path()).unwrap();
    store.add_note(at_root.context.id, ".", "root idea").unwrap();
    let in_src = resolve(&store, &src).unwrap();
    store.add_note(in_src.context.id, ".", "src idea").unwrap();
    assert_ne!(at_root.context.id, in_src.context.id, "two path contexts");

    // First commit -> the repo now has an id.
    common::commit_file(dir.path(), "a.txt", "a");

    let r = resolve(&store, dir.path()).unwrap();
    assert_eq!(r.context.kind, Kind::Repo);
    let report = adopt_if_needed(&mut store, &r).unwrap().expect("adoption");
    assert_eq!(report.notes_moved, 2);
    assert_eq!(report.paths_folded, 2);

    // Both notes now live in the repo context, with correct subpaths.
    let all = store.list_notes(r.context.id, None, true, None).unwrap();
    assert_eq!(all.len(), 2);
    let mut subpaths: Vec<&str> = all.iter().map(|n| n.subpath.as_str()).collect();
    subpaths.sort_unstable();
    assert_eq!(subpaths, vec![".", "src"]);
}

#[test]
fn adoption_is_idempotent() {
    let mut store = Store::open_in_memory().unwrap();
    let dir = common::empty_repo();
    let before = resolve(&store, dir.path()).unwrap();
    store.add_note(before.context.id, ".", "idea").unwrap();
    common::commit_file(dir.path(), "a.txt", "a");

    let r = resolve(&store, dir.path()).unwrap();
    let first = adopt_if_needed(&mut store, &r).unwrap();
    assert!(first.is_some());

    // A second run must find nothing to do, not re-move or duplicate.
    let second = adopt_if_needed(&mut store, &r).unwrap();
    assert!(second.is_none(), "got {second:?}");
    assert_eq!(store.list_notes(r.context.id, None, true, None).unwrap().len(), 1);
}

#[test]
fn adoption_writes_an_audit_row() {
    let mut store = Store::open_in_memory().unwrap();
    let dir = common::empty_repo();
    let before = resolve(&store, dir.path()).unwrap();
    store.add_note(before.context.id, ".", "idea").unwrap();
    common::commit_file(dir.path(), "a.txt", "a");

    let r = resolve(&store, dir.path()).unwrap();
    adopt_if_needed(&mut store, &r).unwrap().unwrap();

    let n: i64 = store
        .conn()
        .query_row("SELECT count(*) FROM adoptions", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 1, "adoption must be auditable for a future --undo");
}

#[test]
fn a_repo_with_no_prior_path_notes_reports_no_adoption() {
    let mut store = Store::open_in_memory().unwrap();
    let dir = common::repo_with_commits(1);
    let r = resolve(&store, dir.path()).unwrap();
    assert!(adopt_if_needed(&mut store, &r).unwrap().is_none());
}
