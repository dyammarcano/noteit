use noteit::render::{parse_short_id, render_list, short_id};
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

#[test]
fn render_list_handles_empty() {
    let out = render_list(&[], None, 0);
    assert!(out.contains("no notes"), "{out}");
}
