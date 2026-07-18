//! Property / fuzz-style tests (std-only, deterministic — no proptest crate).
//!
//! Two invariants that must hold for *any* input:
//! 1. `Store::search` never surfaces a SQL error, regardless of how malformed
//!    the raw query is (the whole point of `sanitize_fts_query`).
//! 2. `cli::parse` never panics for any argument vector — it always returns
//!    `Ok`/`Err`, never `unreachable!`/index-out-of-bounds.

mod common;

use noteit::cli::parse;
use noteit::store::Store;
use noteit::store::notes::sanitize_fts_query;

/// A tiny deterministic PRNG (SplitMix-ish LCG) so the corpus is reproducible
/// across runs without pulling in `rand`.
struct Lcg(u64);
impl Lcg {
    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }
    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[(self.next_u64() as usize) % xs.len()]
    }
}

/// Adversarial fragments: FTS operators, quotes, shell-ish flags, unicode.
const FRAGS: &[&str] = &[
    "\"", "\"\"", "AND", "OR", "NOT", "NEAR", "*", ":", "(", ")", "foo", "bar", "'", "\\", "#tag",
    "-g", "--global", "", " ", "\t", "él", "😀", "^", ";", "--", "add", "search", "list", "delete",
    "plugin", "1a",
];

/// Build a pseudo-random query/arg string from 0..6 fragments.
fn gen_string(rng: &mut Lcg) -> String {
    let n = (rng.next_u64() % 6) as usize;
    let mut parts = Vec::with_capacity(n);
    for _ in 0..n {
        parts.push((*rng.pick(FRAGS)).to_string());
    }
    let sep = if rng.next_u64().is_multiple_of(2) { " " } else { "" };
    parts.join(sep)
}

#[test]
fn search_never_errors_on_arbitrary_input() {
    let repo = common::repo_with_commits(1);
    let store = Store::open_in_memory().unwrap();
    let ctx = noteit::context::resolve(&store, repo.path())
        .unwrap()
        .context;
    store
        .add_note(ctx.id, ".", "a searchable note about tokenizers #urgent")
        .unwrap();

    // Empty / whitespace queries short-circuit to zero results, never an error.
    for q in ["", "   ", "\t\n"] {
        let (rows, total) = store.search(q, None, Some(50)).expect("empty query is Ok");
        assert!(
            rows.is_empty() && total == 0,
            "empty query {q:?} should be empty"
        );
    }

    let mut rng = Lcg(0x9E3779B97F4A7C15);
    for _ in 0..1000 {
        let q = gen_string(&mut rng);
        // The invariant: no matter how malformed, this is Ok, never Err.
        let res = store.search(&q, None, Some(50));
        assert!(
            res.is_ok(),
            "search errored on query {q:?}: {:?}",
            res.err()
        );
        // Local-scope search too (different SQL path).
        let res_scoped = store.search(&q, Some(ctx.id), Some(50));
        assert!(res_scoped.is_ok(), "scoped search errored on {q:?}");
    }
}

#[test]
fn sanitize_fts_query_always_balances_quotes() {
    let mut rng = Lcg(0xD1B54A32D192ED03);
    for _ in 0..1000 {
        let raw = gen_string(&mut rng);
        let sanitized = sanitize_fts_query(&raw);
        let quotes = sanitized.matches('"').count();
        assert!(
            quotes.is_multiple_of(2),
            "unbalanced quotes for {raw:?} -> {sanitized:?}"
        );
        // A blank input yields a blank expression (which callers short-circuit).
        if raw.split_whitespace().next().is_none() {
            assert!(sanitized.is_empty(), "blank input should sanitize to empty");
        }
    }
}

#[test]
fn parse_never_panics_on_arbitrary_args() {
    let mut rng = Lcg(0x2545F4914F6CDD1D);
    for _ in 0..2000 {
        let n = (rng.next_u64() % 5) as usize;
        let argv: Vec<String> = (0..n).map(|_| (*rng.pick(FRAGS)).to_string()).collect();
        // The only requirement: this returns without panicking.
        let _ = parse(&argv);
    }
    // Every single fragment as a lone argument, too.
    for f in FRAGS {
        let _ = parse(&[(*f).to_string()]);
    }
}
