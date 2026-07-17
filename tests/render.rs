use noteit::render::{parse_short_id, render_flat, render_grouped, render_list, short_id};
use noteit::store::contexts::{Context, Kind};
use noteit::store::notes::{Note, Status};

fn note(id: i64, body: &str) -> Note {
    Note {
        id,
        context_id: 1,
        subpath: ".".into(),
        body: body.into(),
        status: Status::Open,
        created_at: 0,
        updated_at: 0,
    }
}

#[test]
fn short_ids_round_trip() {
    for id in [1i64, 35, 36, 1000, 99999] {
        assert_eq!(parse_short_id(&short_id(id)), Some(id), "id {id}");
    }
}

#[test]
fn short_ids_are_base36_and_compact() {
    assert_eq!(short_id(1), "1");
    assert_eq!(short_id(35), "z");
    assert_eq!(short_id(36), "10");
}

#[test]
fn parse_short_id_rejects_garbage() {
    assert_eq!(parse_short_id("!!"), None);
    assert_eq!(parse_short_id(""), None);
}

#[test]
fn render_list_shows_body_and_short_id() {
    let out = render_list(&[note(1, "fix the tokenizer")], None, 1);
    assert!(out.contains("fix the tokenizer"), "{out}");
    assert!(out.contains('1'), "{out}");
}

#[test]
fn render_list_announces_truncation() {
    // Silent truncation would read as completeness. It must not be silent.
    let notes: Vec<Note> = (1..=50).map(|i| note(i, "x")).collect();
    let out = render_list(&notes, Some(50), 390);
    assert!(out.contains("340 more"), "{out}");
    assert!(out.contains("--limit 0"), "{out}");
}

#[test]
fn render_list_is_quiet_when_nothing_is_truncated() {
    let out = render_list(&[note(1, "x")], Some(50), 1);
    assert!(!out.contains("more"), "{out}");
}

#[test]
fn render_list_announces_truncation_even_below_the_limit() {
    // Truncation notice must appear whenever rows were dropped, even if shown < limit.
    // The old guard (shown >= limit) wrongly suppressed the notice in this case.
    let out = render_list(&[note(1, "x")], Some(50), 5);
    assert!(out.contains("4 more"), "{out}");
}

fn context(id: i64, display_name: &str) -> Context {
    Context {
        id,
        kind: Kind::Repo,
        key: format!("key-{id}"),
        display_name: display_name.to_string(),
        name_overridden: false,
        root_path: format!("/repo-{id}"),
        shallow_warned: false,
    }
}

fn note_at(id: i64, created_at: i64, body: &str) -> Note {
    Note {
        id,
        context_id: 1,
        subpath: ".".into(),
        body: body.into(),
        status: Status::Open,
        created_at,
        updated_at: created_at,
    }
}

#[test]
fn render_grouped_keeps_same_named_contexts_separate() {
    // Two DISTINCT contexts (different id, different root_path) that happen
    // to share the same display_name (e.g. two repos both named "app").
    // render_grouped requires rows sorted so each context's rows are
    // contiguous -- pre-sort here using the (display_name, ctx.id,
    // created_at desc) key from cli.rs's --global branches.
    let ctx_a = context(10, "app");
    let ctx_b = context(20, "app");

    let mut rows = vec![
        (ctx_a.clone(), note_at(1, 100, "note a1")),
        (ctx_b.clone(), note_at(2, 200, "note b1")),
        (ctx_a.clone(), note_at(3, 300, "note a2")),
        (ctx_b.clone(), note_at(4, 50, "note b2")),
    ];
    rows.sort_by(|x, y| {
        x.0.display_name
            .cmp(&y.0.display_name)
            .then(x.0.id.cmp(&y.0.id))
            .then(y.1.created_at.cmp(&x.1.created_at))
    });

    let out = render_grouped(&rows, rows.len(), None);

    // Count header lines: a line that equals the display_name exactly
    // (headers are un-indented; note lines are indented with "  [").
    let header_count = out.lines().filter(|l| *l == "app").count();
    assert_eq!(
        header_count, 2,
        "expected exactly two separate 'app' headers (one per distinct context), got: {out}"
    );
}

#[test]
fn render_list_handles_empty() {
    let out = render_list(&[], None, 0);
    assert!(out.contains("no notes"), "{out}");
}

#[test]
fn render_flat_handles_empty() {
    let out = render_flat(&[], 0, None);
    assert!(out.contains("no notes"), "{out}");
}

#[test]
fn render_flat_shows_context_name_and_body_inline() {
    let ctx_a = context(10, "app-one");
    let ctx_b = context(20, "app-two");
    let rows = vec![
        (ctx_a, note_at(1, 100, "fix the tokenizer")),
        (ctx_b, note_at(2, 200, "write docs")),
    ];

    let out = render_flat(&rows, rows.len(), None);
    assert!(out.contains("app-one"), "{out}");
    assert!(out.contains("fix the tokenizer"), "{out}");
    assert!(out.contains("app-two"), "{out}");
    assert!(out.contains("write docs"), "{out}");
    // Flat rows are not grouped -- no bare-header line for the context name.
    assert!(!out.lines().any(|l| l == "app-one"), "{out}");
}

#[test]
fn render_flat_announces_truncation() {
    let ctx = context(1, "app");
    let rows: Vec<_> = (1..=10)
        .map(|i| (ctx.clone(), note_at(i, i, "x")))
        .collect();

    let out = render_flat(&rows, 25, Some(10));
    assert!(out.contains("15 more"), "{out}");
    assert!(out.contains("--limit 0"), "{out}");
}

#[test]
fn render_flat_is_quiet_when_nothing_is_truncated() {
    let ctx = context(1, "app");
    let rows = vec![(ctx, note_at(1, 1, "x"))];
    let out = render_flat(&rows, 1, Some(50));
    assert!(!out.contains("more"), "{out}");
}

#[test]
fn render_grouped_handles_empty() {
    let out = render_grouped(&[], 0, None);
    assert!(out.contains("no notes"), "{out}");
}
