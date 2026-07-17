use noteit::store::Store;
use noteit::store::contexts::Kind;

#[test]
fn open_applies_all_migrations() {
    let store = Store::open_in_memory().expect("open");
    let v: i64 = store
        .conn()
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .expect("user_version");
    assert_eq!(v, 1, "schema should be at version 1");
}

#[test]
fn migrations_are_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("noteit.db");
    Store::open(&path).expect("first open");
    let store = Store::open(&path).expect("second open must not re-apply");
    let v: i64 = store
        .conn()
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(v, 1);
}

#[test]
fn expected_tables_exist() {
    let store = Store::open_in_memory().expect("open");
    for t in ["contexts", "notes", "tags", "note_tags", "adoptions", "notes_fts"] {
        let n: i64 = store
            .conn()
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE name = ?1",
                [t],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1, "table {t} missing");
    }
}

#[test]
fn upsert_context_is_idempotent_on_kind_and_key() {
    let store = Store::open_in_memory().unwrap();
    let a = store
        .upsert_context(Kind::Repo, "urn:noteit:v1:abc", "noteit", "D:\\rust\\noteit")
        .unwrap();
    let b = store
        .upsert_context(Kind::Repo, "urn:noteit:v1:abc", "noteit", "D:\\rust\\noteit")
        .unwrap();
    assert_eq!(a.id, b.id);
}

#[test]
fn rename_survives_a_later_upsert() {
    // The override must win -- otherwise a new note re-derives the
    // basename and silently reverts the rename.
    let store = Store::open_in_memory().unwrap();
    let c = store
        .upsert_context(Kind::Repo, "urn:noteit:v1:abc", "noteit", "D:\\rust\\noteit")
        .unwrap();
    store.rename_context(c.id, "My Project").unwrap();
    let again = store
        .upsert_context(Kind::Repo, "urn:noteit:v1:abc", "noteit", "D:\\rust\\noteit")
        .unwrap();
    assert_eq!(again.display_name, "My Project");
    assert!(again.name_overridden);
}

#[test]
fn path_contexts_under_finds_descendants_only() {
    let store = Store::open_in_memory().unwrap();
    store.upsert_context(Kind::Path, "D:\\rust\\noteit", "noteit", "D:\\rust\\noteit").unwrap();
    store.upsert_context(Kind::Path, "D:\\rust\\noteit\\src", "src", "D:\\rust\\noteit\\src").unwrap();
    store.upsert_context(Kind::Path, "D:\\rust\\other", "other", "D:\\rust\\other").unwrap();

    let found = store.path_contexts_under("D:\\rust\\noteit").unwrap();
    let keys: Vec<&str> = found.iter().map(|c| c.key.as_str()).collect();
    assert_eq!(keys.len(), 2, "got {keys:?}");
    assert!(keys.contains(&"D:\\rust\\noteit"));
    assert!(keys.contains(&"D:\\rust\\noteit\\src"));
    assert!(!keys.contains(&"D:\\rust\\other"));
}

#[test]
fn path_contexts_under_excludes_adjacent_prefix_siblings() {
    // This test verifies that path_contexts_under() correctly excludes sibling directories
    // that share a string prefix but are NOT true descendants. The dangerous case is a
    // directory like "D:\rust\noteit-other" that starts with the root's key but lacks a
    // path separator, which a naive LIKE pattern would incorrectly match.
    // If this breaks, a future feature that folds notes by project path would silently
    // swallow a different project's notes.
    let store = Store::open_in_memory().unwrap();
    store.upsert_context(Kind::Path, "D:\\rust\\noteit", "noteit", "D:\\rust\\noteit").unwrap();
    store.upsert_context(Kind::Path, "D:\\rust\\noteit\\src", "src", "D:\\rust\\noteit\\src").unwrap();
    store.upsert_context(Kind::Path, "D:\\rust\\noteit-other", "noteit-other", "D:\\rust\\noteit-other").unwrap();
    store.upsert_context(Kind::Path, "D:\\rust\\noteitXother", "noteitXother", "D:\\rust\\noteitXother").unwrap();

    let found = store.path_contexts_under("D:\\rust\\noteit").unwrap();
    let keys: Vec<&str> = found.iter().map(|c| c.key.as_str()).collect();
    assert_eq!(keys.len(), 2, "got {keys:?}");
    assert!(keys.contains(&"D:\\rust\\noteit"), "root not found in {keys:?}");
    assert!(keys.contains(&"D:\\rust\\noteit\\src"), "descendant not found in {keys:?}");
    assert!(!keys.contains(&"D:\\rust\\noteit-other"), "adjacent-prefix sibling leaked in {keys:?}");
    assert!(!keys.contains(&"D:\\rust\\noteitXother"), "prefix+char sibling leaked in {keys:?}");
}
