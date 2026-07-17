mod common;

use noteit::cli::{Invocation, ListArgs, parse, run_core};
use noteit::store::Store;
use noteit::store::notes::Status;

fn out_str(buf: Vec<u8>) -> String {
    String::from_utf8(buf).expect("utf8 output")
}

fn args(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[test]
fn capture_saves_a_note_and_exits_zero() {
    let repo = common::repo_with_commits(1);
    let mut store = Store::open_in_memory().unwrap();
    let mut buf = Vec::new();

    let inv = parse(&args(&["fix the tokenizer"])).unwrap();
    let code = run_core(inv, &mut store, repo.path(), &mut buf).unwrap();

    assert_eq!(code, 0);
    let s = out_str(buf);
    assert!(s.contains("saved"), "unexpected output: {s}");

    let notes = store.list_notes(1, None, true, None).unwrap();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].body, "fix the tokenizer");
}

#[test]
fn empty_capture_saves_nothing_and_exits_two() {
    let repo = common::repo_with_commits(1);
    let mut store = Store::open_in_memory().unwrap();
    let mut buf = Vec::new();

    let inv = Invocation::Capture("   ".to_string());
    let code = run_core(inv, &mut store, repo.path(), &mut buf).unwrap();

    assert_eq!(code, 2);
    assert!(out_str(buf).is_empty());
    let notes = store.list_all_notes(true, None).unwrap();
    assert!(notes.is_empty());
}

#[test]
fn list_current_context_shows_saved_notes() {
    let repo = common::repo_with_commits(1);
    let mut store = Store::open_in_memory().unwrap();

    let mut buf = Vec::new();
    run_core(
        Invocation::Capture("alpha task".to_string()),
        &mut store,
        repo.path(),
        &mut buf,
    )
    .unwrap();

    let mut buf = Vec::new();
    let code = run_core(
        Invocation::List(ListArgs::default()),
        &mut store,
        repo.path(),
        &mut buf,
    )
    .unwrap();
    assert_eq!(code, 0);
    let s = out_str(buf);
    assert!(s.contains("alpha task"), "unexpected output: {s}");
}

#[test]
fn list_global_shows_notes_from_other_contexts() {
    let repo_a = common::repo_with_commits(1);
    let repo_b = common::repo_with_commits(1);
    let mut store = Store::open_in_memory().unwrap();

    let mut buf = Vec::new();
    run_core(
        Invocation::Capture("in repo a".to_string()),
        &mut store,
        repo_a.path(),
        &mut buf,
    )
    .unwrap();

    let mut buf = Vec::new();
    let code = run_core(
        Invocation::List(ListArgs {
            global: true,
            ..Default::default()
        }),
        &mut store,
        repo_b.path(),
        &mut buf,
    )
    .unwrap();
    assert_eq!(code, 0);
    let s = out_str(buf);
    assert!(s.contains("in repo a"), "unexpected output: {s}");
}

#[test]
fn list_flat_renders_flat_format() {
    let repo = common::repo_with_commits(1);
    let mut store = Store::open_in_memory().unwrap();

    let mut buf = Vec::new();
    run_core(
        Invocation::Capture("flat note".to_string()),
        &mut store,
        repo.path(),
        &mut buf,
    )
    .unwrap();

    let mut buf = Vec::new();
    let code = run_core(
        Invocation::List(ListArgs {
            global: true,
            flat: true,
            ..Default::default()
        }),
        &mut store,
        repo.path(),
        &mut buf,
    )
    .unwrap();
    assert_eq!(code, 0);
    assert!(out_str(buf).contains("flat note"));
}

#[test]
fn list_by_tag_finds_tagged_notes() {
    let repo = common::repo_with_commits(1);
    let mut store = Store::open_in_memory().unwrap();

    let mut buf = Vec::new();
    run_core(
        Invocation::Capture("tagged #urgent note".to_string()),
        &mut store,
        repo.path(),
        &mut buf,
    )
    .unwrap();

    let mut buf = Vec::new();
    let code = run_core(
        Invocation::List(ListArgs {
            tag: Some("urgent".to_string()),
            ..Default::default()
        }),
        &mut store,
        repo.path(),
        &mut buf,
    )
    .unwrap();
    assert_eq!(code, 0);
    assert!(out_str(buf).contains("tagged"));
}

#[test]
fn list_all_includes_done_notes() {
    let repo = common::repo_with_commits(1);
    let mut store = Store::open_in_memory().unwrap();
    let ctx = noteit::context::resolve(&store, repo.path())
        .unwrap()
        .context;
    let n = store.add_note(ctx.id, ".", "already done").unwrap();
    store.set_status(n.id, Status::Done).unwrap();

    let mut buf = Vec::new();
    let code = run_core(
        Invocation::List(ListArgs {
            all: true,
            ..Default::default()
        }),
        &mut store,
        repo.path(),
        &mut buf,
    )
    .unwrap();
    assert_eq!(code, 0);
    assert!(out_str(buf).contains("already done"));
}

#[test]
fn list_limit_zero_shows_everything() {
    let repo = common::repo_with_commits(1);
    let mut store = Store::open_in_memory().unwrap();
    for i in 0..3 {
        let mut buf = Vec::new();
        run_core(
            Invocation::Capture(format!("note {i}")),
            &mut store,
            repo.path(),
            &mut buf,
        )
        .unwrap();
    }

    let mut buf = Vec::new();
    let code = run_core(
        Invocation::List(ListArgs {
            limit: Some(0),
            ..Default::default()
        }),
        &mut store,
        repo.path(),
        &mut buf,
    )
    .unwrap();
    assert_eq!(code, 0);
    let s = out_str(buf);
    for i in 0..3 {
        assert!(s.contains(&format!("note {i}")), "missing note {i}: {s}");
    }
    assert!(!s.contains("more (--limit"));
}

#[test]
fn search_local_finds_matching_note() {
    let repo = common::repo_with_commits(1);
    let mut store = Store::open_in_memory().unwrap();

    let mut buf = Vec::new();
    run_core(
        Invocation::Capture("searchable tokenizer bug".to_string()),
        &mut store,
        repo.path(),
        &mut buf,
    )
    .unwrap();

    let mut buf = Vec::new();
    let code = run_core(
        Invocation::Search {
            query: "tokenizer".to_string(),
            global: false,
        },
        &mut store,
        repo.path(),
        &mut buf,
    )
    .unwrap();
    assert_eq!(code, 0);
    assert!(out_str(buf).contains("searchable tokenizer bug"));
}

#[test]
fn search_global_finds_note_from_other_context() {
    let repo_a = common::repo_with_commits(1);
    let repo_b = common::repo_with_commits(1);
    let mut store = Store::open_in_memory().unwrap();

    let mut buf = Vec::new();
    run_core(
        Invocation::Capture("global search target".to_string()),
        &mut store,
        repo_a.path(),
        &mut buf,
    )
    .unwrap();

    let mut buf = Vec::new();
    let code = run_core(
        Invocation::Search {
            query: "target".to_string(),
            global: true,
        },
        &mut store,
        repo_b.path(),
        &mut buf,
    )
    .unwrap();
    assert_eq!(code, 0);
    assert!(out_str(buf).contains("global search target"));
}

#[test]
fn set_status_done_and_open_succeed() {
    let repo = common::repo_with_commits(1);
    let mut store = Store::open_in_memory().unwrap();
    let ctx = noteit::context::resolve(&store, repo.path())
        .unwrap()
        .context;
    let n = store.add_note(ctx.id, ".", "toggle me").unwrap();
    let id = noteit::render::short_id(n.id);

    let mut buf = Vec::new();
    let code = run_core(
        Invocation::SetStatus {
            id: id.clone(),
            status: Status::Done,
        },
        &mut store,
        repo.path(),
        &mut buf,
    )
    .unwrap();
    assert_eq!(code, 0);
    assert!(out_str(buf).contains("-> done"));

    let mut buf = Vec::new();
    let code = run_core(
        Invocation::SetStatus {
            id,
            status: Status::Open,
        },
        &mut store,
        repo.path(),
        &mut buf,
    )
    .unwrap();
    assert_eq!(code, 0);
    assert!(out_str(buf).contains("-> open"));
}

#[test]
fn set_status_not_found_id_exits_one() {
    let repo = common::repo_with_commits(1);
    let mut store = Store::open_in_memory().unwrap();

    let mut buf = Vec::new();
    let code = run_core(
        Invocation::SetStatus {
            id: "zz".to_string(),
            status: Status::Done,
        },
        &mut store,
        repo.path(),
        &mut buf,
    )
    .unwrap();
    assert_eq!(code, 1);
}

#[test]
fn set_status_invalid_id_exits_two() {
    let repo = common::repo_with_commits(1);
    let mut store = Store::open_in_memory().unwrap();

    let mut buf = Vec::new();
    let code = run_core(
        Invocation::SetStatus {
            id: "not-base36!".to_string(),
            status: Status::Done,
        },
        &mut store,
        repo.path(),
        &mut buf,
    )
    .unwrap();
    assert_eq!(code, 2);
}

#[test]
fn delete_success_exit_0() {
    let repo = common::repo_with_commits(1);
    let mut store = Store::open_in_memory().unwrap();
    let ctx = noteit::context::resolve(&store, repo.path())
        .unwrap()
        .context;
    let n = store.add_note(ctx.id, ".", "delete me please").unwrap();
    let id = noteit::render::short_id(n.id);

    let mut buf = Vec::new();
    let code = run_core(Invocation::Delete { id }, &mut store, repo.path(), &mut buf).unwrap();
    assert_eq!(code, 0);
    let s = out_str(buf);
    assert!(s.contains("deleted"), "unexpected output: {s}");

    let notes = store.list_notes(ctx.id, None, true, None).unwrap();
    assert!(notes.is_empty());
}

#[test]
fn delete_not_found_exit_1() {
    let repo = common::repo_with_commits(1);
    let mut store = Store::open_in_memory().unwrap();

    let mut buf = Vec::new();
    let code = run_core(
        Invocation::Delete {
            id: "zz".to_string(),
        },
        &mut store,
        repo.path(),
        &mut buf,
    )
    .unwrap();
    assert_eq!(code, 1);
}

#[test]
fn delete_invalid_id_exit_2() {
    let repo = common::repo_with_commits(1);
    let mut store = Store::open_in_memory().unwrap();

    let mut buf = Vec::new();
    let code = run_core(
        Invocation::Delete {
            id: "not-base36!".to_string(),
        },
        &mut store,
        repo.path(),
        &mut buf,
    )
    .unwrap();
    assert_eq!(code, 2);
}

#[test]
fn rename_updates_display_name() {
    let repo = common::repo_with_commits(1);
    let mut store = Store::open_in_memory().unwrap();

    let mut buf = Vec::new();
    let code = run_core(
        Invocation::Rename("My Project".to_string()),
        &mut store,
        repo.path(),
        &mut buf,
    )
    .unwrap();
    assert_eq!(code, 0);
    assert!(out_str(buf).contains("renamed to My Project"));
}

#[test]
fn adopt_undo_with_nothing_to_undo_exits_zero() {
    let repo = common::repo_with_commits(1);
    let mut store = Store::open_in_memory().unwrap();

    let mut buf = Vec::new();
    let code = run_core(
        Invocation::Adopt { undo: true },
        &mut store,
        repo.path(),
        &mut buf,
    )
    .unwrap();
    assert_eq!(code, 0);
    assert!(out_str(buf).contains("nothing to undo"));
}

#[test]
fn adopt_undo_reverses_a_real_adoption() {
    // A path-bound note followed by that same directory becoming a repo
    // triggers automatic adoption on the next run_core call. `adopt --undo`
    // must then fold it back to path-bound.
    let repo = common::empty_repo();
    let mut store = Store::open_in_memory().unwrap();

    // First: capture while the dir has zero commits (NoCommits -> path bound).
    let mut buf = Vec::new();
    run_core(
        Invocation::Capture("pre-repo note".to_string()),
        &mut store,
        repo.path(),
        &mut buf,
    )
    .unwrap();

    // Now give it a commit so it resolves as a real repo -- the next
    // run_core call auto-adopts the path context's note.
    common::commit_file(repo.path(), "f0.txt", "v0");
    let mut buf = Vec::new();
    run_core(
        Invocation::List(ListArgs::default()),
        &mut store,
        repo.path(),
        &mut buf,
    )
    .unwrap();

    // Confirm adoption happened: the note is now visible in repo scope.
    let s = out_str(buf);
    assert!(s.contains("pre-repo note"), "note missing after adopt: {s}");

    let mut buf = Vec::new();
    let code = run_core(
        Invocation::Adopt { undo: true },
        &mut store,
        repo.path(),
        &mut buf,
    )
    .unwrap();
    assert_eq!(code, 0);
    let s = out_str(buf);
    assert!(s.contains("un-adopted"), "unexpected undo output: {s}");
}
