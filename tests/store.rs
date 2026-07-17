use noteit::store::Store;
use noteit::store::contexts::Kind;
use noteit::store::notes::{parse_tags, sanitize_fts_query, Status};

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

fn seed_ctx(store: &Store) -> i64 {
    store
        .upsert_context(Kind::Repo, "urn:noteit:v1:abc", "noteit", "D:\\rust\\noteit")
        .unwrap()
        .id
}

#[test]
fn parse_tags_extracts_hashtags() {
    assert_eq!(parse_tags("fix the #fts5 tokenizer #perf"), vec!["fts5", "perf"]);
    assert_eq!(parse_tags("no tags here"), Vec::<String>::new());
    // Deduped, lowercased.
    assert_eq!(parse_tags("#Perf and #perf"), vec!["perf"]);
}

#[test]
fn add_note_keeps_tags_in_body_and_in_table() {
    let store = Store::open_in_memory().unwrap();
    let ctx = seed_ctx(&store);
    let n = store.add_note(ctx, ".", "fix the #fts5 tokenizer").unwrap();
    // Body keeps the tag for display fidelity.
    assert!(n.body.contains("#fts5"));
    // Table drives queries.
    let found = store.notes_by_tag("fts5", Some(ctx), true, None).unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].1.id, n.id);
}

#[test]
fn notes_by_tag_hides_done_by_default() {
    let store = Store::open_in_memory().unwrap();
    let ctx = seed_ctx(&store);
    let a = store.add_note(ctx, ".", "open one #bug").unwrap();
    let b = store.add_note(ctx, ".", "done one #bug").unwrap();
    store.set_status(b.id, Status::Done).unwrap();

    let open_only = store.notes_by_tag("bug", Some(ctx), false, None).unwrap();
    assert_eq!(open_only.len(), 1);
    assert_eq!(open_only[0].1.id, a.id);

    let all = store.notes_by_tag("bug", Some(ctx), true, None).unwrap();
    assert_eq!(all.len(), 2);
}

#[test]
fn notes_by_tag_respects_limit() {
    let store = Store::open_in_memory().unwrap();
    let ctx = seed_ctx(&store);
    for _ in 0..5 {
        store.add_note(ctx, ".", "note #bug").unwrap();
    }
    let limited = store.notes_by_tag("bug", Some(ctx), true, Some(2)).unwrap();
    assert_eq!(limited.len(), 2);
    let unlimited = store.notes_by_tag("bug", Some(ctx), true, None).unwrap();
    assert_eq!(unlimited.len(), 5);
}

#[test]
fn list_notes_hides_done_by_default() {
    let store = Store::open_in_memory().unwrap();
    let ctx = seed_ctx(&store);
    let a = store.add_note(ctx, ".", "open one").unwrap();
    let b = store.add_note(ctx, ".", "done one").unwrap();
    store.set_status(b.id, Status::Done).unwrap();

    let open = store.list_notes(ctx, None, false, None).unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].id, a.id);

    let all = store.list_notes(ctx, None, true, None).unwrap();
    assert_eq!(all.len(), 2);
}

#[test]
fn list_notes_filters_by_subpath() {
    let store = Store::open_in_memory().unwrap();
    let ctx = seed_ctx(&store);
    store.add_note(ctx, ".", "at root").unwrap();
    store.add_note(ctx, "src", "in src").unwrap();

    let in_src = store.list_notes(ctx, Some("src"), false, None).unwrap();
    assert_eq!(in_src.len(), 1);
    assert_eq!(in_src[0].body, "in src");
}

#[test]
fn search_finds_by_body_and_scopes_by_context() {
    let store = Store::open_in_memory().unwrap();
    let a = seed_ctx(&store);
    let b = store
        .upsert_context(Kind::Repo, "urn:noteit:v1:def", "other", "D:\\rust\\other")
        .unwrap()
        .id;
    store.add_note(a, ".", "tokenizer bug").unwrap();
    store.add_note(b, ".", "tokenizer elsewhere").unwrap();

    assert_eq!(store.search("tokenizer", None, None).unwrap().len(), 2);
    assert_eq!(store.search("tokenizer", Some(a), None).unwrap().len(), 1);
    assert_eq!(store.search("nonexistent", None, None).unwrap().len(), 0);
}

#[test]
fn search_reflects_edits_via_fts_triggers() {
    let store = Store::open_in_memory().unwrap();
    let ctx = seed_ctx(&store);
    let n = store.add_note(ctx, ".", "findme").unwrap();
    assert_eq!(store.search("findme", None, None).unwrap().len(), 1);
    // set_status touches the row; the AFTER UPDATE trigger must keep the
    // FTS index consistent rather than duplicating the entry.
    store.set_status(n.id, Status::Done).unwrap();
    assert_eq!(store.search("findme", None, None).unwrap().len(), 1);
}

#[test]
fn search_with_unbalanced_quote_does_not_error() {
    let store = Store::open_in_memory().unwrap();
    let ctx = seed_ctx(&store);
    store.add_note(ctx, ".", "has a quote mark").unwrap();

    let result = store.search("\"foo", None, None);
    assert!(result.is_ok(), "unbalanced quote must not error: {result:?}");
}

#[test]
fn search_with_bare_boolean_operator_is_literal() {
    let store = Store::open_in_memory().unwrap();
    let ctx = seed_ctx(&store);
    let n = store.add_note(ctx, ".", "salt AND pepper").unwrap();

    let found = store.search("AND", None, None).unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].1.id, n.id);
}

#[test]
fn search_multiple_words_requires_all_of_them() {
    let store = Store::open_in_memory().unwrap();
    let ctx = seed_ctx(&store);
    let a = store.add_note(ctx, ".", "tokenizer bug here").unwrap();
    store.add_note(ctx, ".", "tokenizer only").unwrap();

    let found = store.search("tokenizer bug", None, None).unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].1.id, a.id);
}

#[test]
fn search_empty_query_returns_no_results() {
    let store = Store::open_in_memory().unwrap();
    seed_ctx(&store);

    let found = store.search("   ", None, None).unwrap();
    assert!(found.is_empty());
}

#[test]
fn sanitize_fts_query_quotes_tokens_and_handles_edge_cases() {
    assert_eq!(sanitize_fts_query("foo bar"), "\"foo\" \"bar\"");
    assert_eq!(sanitize_fts_query("a\"b"), "\"a\"\"b\"");
    assert_eq!(sanitize_fts_query("   "), "");
}

#[test]
fn set_status_reports_whether_a_row_matched() {
    let store = Store::open_in_memory().unwrap();
    let ctx = seed_ctx(&store);
    let n = store.add_note(ctx, ".", "x").unwrap();
    assert!(store.set_status(n.id, Status::Done).unwrap());
    assert!(!store.set_status(99999, Status::Done).unwrap());
}
