use noteit::cli::{parse, Invocation};
use noteit::store::notes::Status;

fn args(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
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
fn text_merely_starting_with_a_verb_word_is_still_captured() {
    // "search this" -- first token is a verb, so per the rule it dispatches.
    // Documenting the sharp edge: users needing literal text use `add`.
    match parse(&args(&["add", "search this"])).unwrap() {
        Invocation::Capture(body) => assert_eq!(body, "search this"),
        other => panic!("got {other:?}"),
    }
}

#[test]
fn list_flags_parse() {
    match parse(&args(&["list", "--global", "--flat", "--all", "--limit", "10"])).unwrap() {
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
