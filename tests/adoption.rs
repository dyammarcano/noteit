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
fn adoption_audit_row_captures_the_folded_context_identity() {
    let mut store = Store::open_in_memory().unwrap();
    let dir = common::empty_repo();
    let before = resolve(&store, dir.path()).unwrap();
    store.add_note(before.context.id, ".", "idea").unwrap();
    let from_key = before.context.key.clone();
    let from_root_path = before.context.root_path.clone();
    let from_display_name = before.context.display_name.clone();
    common::commit_file(dir.path(), "a.txt", "a");

    let r = resolve(&store, dir.path()).unwrap();
    adopt_if_needed(&mut store, &r).unwrap().unwrap();

    let (row_key, row_root, row_name): (String, String, String) = store
        .conn()
        .query_row(
            "SELECT from_key, from_root_path, from_display_name FROM adoptions",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(row_key, from_key, "undo needs the original context key");
    assert_eq!(row_root, from_root_path, "undo needs the original root path");
    assert_eq!(row_name, from_display_name, "undo needs the original display name");
}

#[test]
fn empty_path_context_fold_is_reported_and_audited() {
    let mut store = Store::open_in_memory().unwrap();
    let dir = common::empty_repo(); // zero commits -> path context
    let sub = dir.path().join("empty");
    std::fs::create_dir_all(&sub).unwrap();

    // Resolve the subdir to create a path context, but add no notes to it.
    let empty_ctx = resolve(&store, &sub).unwrap();
    assert_eq!(empty_ctx.context.kind, Kind::Path);

    // First commit -> the repo now has an id.
    common::commit_file(dir.path(), "a.txt", "a");

    let r = resolve(&store, dir.path()).unwrap();
    assert_eq!(r.context.kind, Kind::Repo);
    let report = adopt_if_needed(&mut store, &r).unwrap();
    let report = report.expect("empty fold must still be reported");
    assert!(report.paths_folded >= 1);
    assert_eq!(report.notes_moved, 0);

    let n: i64 = store
        .conn()
        .query_row("SELECT count(*) FROM adoptions", [], |r| r.get(0))
        .unwrap();
    assert!(n >= 1, "empty fold must still be audited");
}

#[test]
fn a_repo_with_no_prior_path_notes_reports_no_adoption() {
    let mut store = Store::open_in_memory().unwrap();
    let dir = common::repo_with_commits(1);
    let r = resolve(&store, dir.path()).unwrap();
    assert!(adopt_if_needed(&mut store, &r).unwrap().is_none());
}

#[test]
fn shallow_nested_repo_is_not_adopted() {
    // A shallow-cloned submodule has no computable repo id (project_id
    // fails with Shallow), but it IS a separate repository and must not
    // have its notes swallowed by the parent when the parent adopts.
    let origin = common::repo_with_commits(2);
    let parent = common::repo_with_commits(1);

    let vendor_path = parent.path().join("vendor");
    let url = format!("file:///{}", origin.path().to_str().unwrap().replace('\\', "/"));
    common::git(
        parent.path(),
        &["clone", "-q", "--depth", "1", &url, vendor_path.to_str().unwrap()],
    );

    // Prove the fixture is genuinely shallow before relying on it.
    assert!(
        vendor_path.join(".git").join("shallow").exists(),
        "vendor clone must have a .git/shallow file to be genuinely shallow"
    );
    let err = noteit::repoid::project_id(&vendor_path).unwrap_err();
    assert!(
        matches!(err, noteit::repoid::RepoIdError::Shallow),
        "vendor clone must be identified as Shallow, got {err:?}"
    );

    let mut store = Store::open_in_memory().unwrap();

    // Capture a note inside the shallow nested repo -- it binds to a PATH
    // context because project_id fails.
    let vendor_resolved = resolve(&store, &vendor_path).unwrap();
    assert_eq!(vendor_resolved.context.kind, Kind::Path);
    store.add_note(vendor_resolved.context.id, ".", "vendor idea").unwrap();

    // resolve() canonicalizes, so look the context back up by the
    // canonicalized path, matching the stored key.
    let vendor_canon =
        vendor_path.canonicalize().unwrap_or_else(|_| vendor_path.clone());
    let vendor_key = vendor_canon.to_string_lossy().to_string();

    // Resolve and adopt from the PARENT repo.
    let parent_resolved = resolve(&store, parent.path()).unwrap();
    assert_eq!(parent_resolved.context.kind, Kind::Repo);
    let report = adopt_if_needed(&mut store, &parent_resolved).unwrap();
    assert!(report.is_none(), "vendor note must not be adopted, got {report:?}");

    // The vendor path context must still exist and still hold the note.
    let vendor_ctx = store
        .find_context(Kind::Path, &vendor_key)
        .unwrap()
        .expect("vendor path context must still exist");
    let vendor_notes = store.list_notes(vendor_ctx.id, None, true, None).unwrap();
    assert_eq!(vendor_notes.len(), 1, "vendor note must remain in its own path context");

    // The parent repo context must NOT contain the vendor note.
    let parent_notes =
        store.list_notes(parent_resolved.context.id, None, true, None).unwrap();
    assert!(
        parent_notes.iter().all(|n| n.subpath != "vendor"),
        "vendor note must not have been folded into the parent repo context"
    );
}
