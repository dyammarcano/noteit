use noteit::store::Store;

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
