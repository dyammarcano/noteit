use noteit::cli::{Invocation, VERBS, parse};
use noteit::store::notes::Status;

fn args(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[test]
fn adopt_undo_parses() {
    assert!(matches!(
        parse(&args(&["adopt", "--undo"])).unwrap(),
        Invocation::Adopt { undo: true }
    ));
}

#[test]
fn adopt_without_undo_is_an_error() {
    assert!(parse(&args(&["adopt"])).is_err());
}

#[test]
fn bare_invocation_lists() {
    assert!(matches!(parse(&args(&[])).unwrap(), Invocation::List(_)));
}

#[test]
fn free_text_is_captured() {
    match parse(&args(&["fix the tokenizer"])).unwrap() {
        Invocation::Capture(body) => assert_eq!(body, "fix the tokenizer"),
        other => panic!("got {other:?}"),
    }
}

#[test]
fn unquoted_multiword_text_is_joined_and_captured() {
    match parse(&args(&["fix", "the", "tokenizer"])).unwrap() {
        Invocation::Capture(body) => assert_eq!(body, "fix the tokenizer"),
        other => panic!("got {other:?}"),
    }
}

#[test]
fn a_known_verb_wins_over_capture() {
    // THE ambiguity rule: `noteit search` is the verb, not a note.
    match parse(&args(&["search", "tokenizer"])).unwrap() {
        Invocation::Search { query, global } => {
            assert_eq!(query, "tokenizer");
            assert!(!global);
        }
        other => panic!("got {other:?}"),
    }
}

#[test]
fn add_is_the_escape_hatch_for_verb_colliding_text() {
    match parse(&args(&["add", "search"])).unwrap() {
        Invocation::Capture(body) => assert_eq!(body, "search"),
        other => panic!("got {other:?}"),
    }
}

#[test]
fn bare_text_starting_with_a_verb_dispatches_the_verb_not_capture() {
    // This is the documented cost of the ambiguity rule: bare `noteit search
    // this` (no `add`) dispatches the search verb with query "this" -- it does
    // NOT capture a note reading "search this". Users who want that literal
    // text captured must use the escape hatch: `noteit add "search this"`.
    // Do not "fix" this thinking it's a bug -- it's the deliberate design.
    match parse(&args(&["search", "this"])).unwrap() {
        Invocation::Search { query, global } => {
            assert_eq!(query, "this");
            assert!(!global);
        }
        other => panic!("got {other:?}"),
    }
}

#[test]
fn list_tag_without_a_value_is_an_error() {
    assert!(parse(&args(&["list", "--tag"])).is_err());
}

#[test]
fn every_verb_in_verbs_has_a_match_arm() {
    // Guards against VERBS drifting out of sync with the match arms in
    // `parse`, which would otherwise turn `unreachable!()` into a live panic.
    for verb in VERBS {
        let _ = parse(&args(&[verb, "x"]));
    }
}

#[test]
fn list_flags_parse() {
    match parse(&args(&[
        "list", "--global", "--flat", "--all", "--limit", "10",
    ]))
    .unwrap()
    {
        Invocation::List(a) => {
            assert!(a.global && a.flat && a.all);
            assert_eq!(a.limit, Some(10));
        }
        other => panic!("got {other:?}"),
    }
}

#[test]
fn done_and_open_take_short_ids() {
    match parse(&args(&["done", "1a"])).unwrap() {
        Invocation::SetStatus { id, status } => {
            assert_eq!(id, "1a");
            assert_eq!(status, Status::Done);
        }
        other => panic!("got {other:?}"),
    }
    match parse(&args(&["open", "1a"])).unwrap() {
        Invocation::SetStatus { status, .. } => assert_eq!(status, Status::Open),
        other => panic!("got {other:?}"),
    }
}

#[test]
fn project_rename_parses() {
    match parse(&args(&["project", "rename", "My Project"])).unwrap() {
        Invocation::Rename(name) => assert_eq!(name, "My Project"),
        other => panic!("got {other:?}"),
    }
}

#[test]
fn new_opens_the_editor() {
    assert!(matches!(parse(&args(&["new"])).unwrap(), Invocation::New));
}

#[test]
fn help_flag_is_not_captured_as_a_note() {
    // Regression guard: dropping clap removed help generation, and under the
    // ambiguity rule an unknown first argument is note text. Without this
    // explicit check, `noteit --help` would save a note whose body is
    // literally "--help".
    assert!(matches!(
        parse(&args(&["--help"])).unwrap(),
        Invocation::Help
    ));
    assert!(matches!(parse(&args(&["-h"])).unwrap(), Invocation::Help));
}

#[test]
fn version_flag_is_not_captured_as_a_note() {
    assert!(matches!(
        parse(&args(&["--version"])).unwrap(),
        Invocation::Version
    ));
    assert!(matches!(
        parse(&args(&["-V"])).unwrap(),
        Invocation::Version
    ));
}
