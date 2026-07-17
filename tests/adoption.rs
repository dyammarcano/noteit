mod common;

use noteit::context::{adopt_if_needed, resolve};
use noteit::store::Store;
use noteit::store::contexts::Kind;

#[test]
fn path_notes_fold_into_the_repo_context_preserving_subpaths() {
    let mut store = Store::open_in_memory().unwrap();
    let dir = common::empty_repo(); // zero commits -> path context
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();

    // Capture at the root and in src BEFORE the first commit.
    let at_root = resolve(&store, dir.path()).unwrap();
    store
        .add_note(at_root.context.id, ".", "root idea")
        .unwrap();
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
    assert_eq!(
        store
            .list_notes(r.context.id, None, true, None)
            .unwrap()
            .len(),
        1
    );
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
    assert_eq!(
        row_root, from_root_path,
        "undo needs the original root path"
    );
    assert_eq!(
        row_name, from_display_name,
        "undo needs the original display name"
    );
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
fn adoption_uses_the_live_root_not_a_stale_stored_one() {
    // Regression for I2: upsert_context deliberately never updates an
    // existing row's root_path (that's what protects `project rename` from
    // being reverted). But that means a repo context's stored root_path can
    // go stale relative to where the SAME repo (by urn) is actually checked
    // out today. adopt_if_needed must scan under the LIVE root computed this
    // run, not the stale stored one, or it silently orphans notes.
    let origin = common::repo_with_commits(1);

    // Clone X to location A and resolve there -- this seeds the repo
    // context's root_path = A.
    let a_dir = tempfile::tempdir().unwrap();
    let url = format!(
        "file:///{}",
        origin.path().to_str().unwrap().replace('\\', "/")
    );
    common::git(a_dir.path(), &["clone", "-q", &url, "."]);
    let mut store = Store::open_in_memory().unwrap();
    let resolved_a = resolve(&store, a_dir.path()).unwrap();
    assert_eq!(resolved_a.context.kind, Kind::Repo);
    let a_canon = a_dir.path().canonicalize().unwrap();
    assert_eq!(resolved_a.context.root_path, a_canon.to_string_lossy());

    // Capture a note in a SEPARATE plain directory B (not yet a repo) --
    // this creates a path context keyed at B.
    let b_dir = tempfile::tempdir().unwrap();
    let resolved_b_before = resolve(&store, b_dir.path()).unwrap();
    assert_eq!(resolved_b_before.context.kind, Kind::Path);
    store
        .add_note(resolved_b_before.context.id, ".", "note in B")
        .unwrap();

    // Now turn B into a clone of the SAME repo X -- same root commit, so
    // resolving B will match the SAME urn already seeded (with root_path=A).
    std::fs::remove_dir_all(b_dir.path()).unwrap();
    std::fs::create_dir_all(b_dir.path()).unwrap();
    common::git(b_dir.path(), &["clone", "-q", &url, "."]);

    let resolved_b = resolve(&store, b_dir.path()).unwrap();
    assert_eq!(resolved_b.context.kind, Kind::Repo);
    assert_eq!(
        resolved_b.context.id, resolved_a.context.id,
        "B must resolve to the SAME repo context (same urn) as A"
    );
    // Confirm the precondition the whole test hinges on: the stored
    // root_path is stale (still A), not B -- otherwise this test proves
    // nothing.
    let b_canon = b_dir.path().canonicalize().unwrap();
    assert_ne!(
        resolved_b.context.root_path,
        b_canon.to_string_lossy(),
        "precondition failed: stored root_path must be stale (A), not B"
    );

    let report = adopt_if_needed(&mut store, &resolved_b).unwrap();
    let report = report.expect(
        "the note captured in B must be adopted using the LIVE root (B), \
         not the stale stored root_path (A) -- otherwise it is silently orphaned",
    );
    assert!(report.notes_moved >= 1);
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
    let url = format!(
        "file:///{}",
        origin.path().to_str().unwrap().replace('\\', "/")
    );
    common::git(
        parent.path(),
        &[
            "clone",
            "-q",
            "--depth",
            "1",
            &url,
            vendor_path.to_str().unwrap(),
        ],
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
    store
        .add_note(vendor_resolved.context.id, ".", "vendor idea")
        .unwrap();

    // resolve() canonicalizes, so look the context back up by the
    // canonicalized path, matching the stored key.
    let vendor_canon = vendor_path
        .canonicalize()
        .unwrap_or_else(|_| vendor_path.clone());
    let vendor_key = vendor_canon.to_string_lossy().to_string();

    // Resolve and adopt from the PARENT repo.
    let parent_resolved = resolve(&store, parent.path()).unwrap();
    assert_eq!(parent_resolved.context.kind, Kind::Repo);
    let report = adopt_if_needed(&mut store, &parent_resolved).unwrap();
    assert!(
        report.is_none(),
        "vendor note must not be adopted, got {report:?}"
    );

    // The vendor path context must still exist and still hold the note.
    let vendor_ctx = store
        .find_context(Kind::Path, &vendor_key)
        .unwrap()
        .expect("vendor path context must still exist");
    let vendor_notes = store.list_notes(vendor_ctx.id, None, true, None).unwrap();
    assert_eq!(
        vendor_notes.len(),
        1,
        "vendor note must remain in its own path context"
    );

    // The parent repo context must NOT contain the vendor note.
    let parent_notes = store
        .list_notes(parent_resolved.context.id, None, true, None)
        .unwrap();
    assert!(
        parent_notes.iter().all(|n| n.subpath != "vendor"),
        "vendor note must not have been folded into the parent repo context"
    );
}

#[test]
fn undo_restores_notes_to_a_path_context() {
    let mut store = Store::open_in_memory().unwrap();
    let dir = common::empty_repo();
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();

    let in_src = resolve(&store, &src).unwrap();
    store.add_note(in_src.context.id, ".", "src idea").unwrap();

    common::commit_file(dir.path(), "a.txt", "a");

    let r = resolve(&store, dir.path()).unwrap();
    let report = adopt_if_needed(&mut store, &r).unwrap().expect("adoption");
    assert_eq!(report.notes_moved, 1);

    let undo = store
        .undo_last_adoption(r.context.id)
        .unwrap()
        .expect("undo");
    assert_eq!(undo.notes_restored, 1);

    // Repo context no longer holds the note.
    let repo_notes = store.list_notes(r.context.id, None, true, None).unwrap();
    assert!(
        repo_notes.is_empty(),
        "note must be moved out of the repo context"
    );

    // The note is back in a Kind::Path context with subpath ".".
    let src_canon = src.canonicalize().unwrap();
    let restored_ctx = store
        .find_context(Kind::Path, &src_canon.to_string_lossy())
        .unwrap()
        .expect("restored path context must exist");
    let restored_notes = store.list_notes(restored_ctx.id, None, true, None).unwrap();
    assert_eq!(restored_notes.len(), 1);
    assert_eq!(restored_notes[0].subpath, ".");

    // Audit row is gone.
    let n: i64 = store
        .conn()
        .query_row("SELECT count(*) FROM adoptions", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 0, "undone adoption's audit row must be deleted");
}

#[test]
fn undo_pins_so_auto_adoption_does_not_re_fold() {
    let mut store = Store::open_in_memory().unwrap();
    let dir = common::empty_repo();
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();

    let in_src = resolve(&store, &src).unwrap();
    store.add_note(in_src.context.id, ".", "src idea").unwrap();

    common::commit_file(dir.path(), "a.txt", "a");

    let r = resolve(&store, dir.path()).unwrap();
    adopt_if_needed(&mut store, &r).unwrap().expect("adoption");
    store
        .undo_last_adoption(r.context.id)
        .unwrap()
        .expect("undo");

    // Resolve the repo again and try to auto-adopt -- must be a no-op
    // because the recreated path context is pinned.
    let r2 = resolve(&store, dir.path()).unwrap();
    let report = adopt_if_needed(&mut store, &r2).unwrap();
    assert!(
        report.is_none(),
        "pinned path context must not be re-adopted, got {report:?}"
    );
}

#[test]
fn undo_with_nothing_to_undo_returns_none() {
    let mut store = Store::open_in_memory().unwrap();
    let dir = common::repo_with_commits(1);
    let r = resolve(&store, dir.path()).unwrap();
    assert!(store.undo_last_adoption(r.context.id).unwrap().is_none());
}
