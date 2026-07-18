//! Command-line surface for noteit.
//!
//! [`parse`] and friends turn argv into an [`Invocation`] (see `parse.rs`);
//! [`run`]/[`run_core`] below dispatch it against the store.

mod parse;
pub use parse::*;

use std::io::Write;
use std::path::Path;

use crate::context::{adopt_if_needed, resolve};
use crate::render;
use crate::store::{Store, default_db_path};

const DEFAULT_LIMIT: usize = 50;

/// Whether informational stderr notices (repo-detection warnings, adoption
/// announcements) should be printed. `NOTEIT_QUIET` set to anything other than
/// empty or `0` suppresses them. Hard errors are never suppressed — only the
/// advisory notices that a scripting caller may not want on stderr.
///
/// noteit's verbosity model is intentionally env-driven, not flag-driven: a
/// global `-v/-q` flag would fight the first-arg ambiguity rule (see ADR-0002),
/// so there is a single quiet/normal switch rather than log levels.
fn notices_enabled() -> bool {
    match std::env::var_os("NOTEIT_QUIET") {
        Some(v) => v.is_empty() || v == "0",
        None => true,
    }
}

/// The single sink for informational stderr notices — respects
/// [`notices_enabled`] so every notice is gated in one place. Hard errors do
/// NOT go through here; they always print.
fn notice(args: std::fmt::Arguments<'_>) {
    if notices_enabled() {
        eprintln!("{args}");
    }
}

fn effective_limit(requested: Option<usize>) -> Option<usize> {
    match requested {
        Some(0) => None, // --limit 0 means everything
        Some(n) => Some(n),
        None => Some(DEFAULT_LIMIT),
    }
}

/// Open a note body in $EDITOR (or $VISUAL) via a temp file.
pub fn edit_in_editor() -> Result<String, Box<dyn std::error::Error>> {
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| {
            if cfg!(windows) {
                "notepad".to_string()
            } else {
                "vi".to_string()
            }
        });
    // A NamedTempFile is created with an unpredictable name and exclusive
    // access, unlike the old `noteit-<pid>.md` path -- predictable temp
    // paths are a symlink/collision hazard on shared systems. Convert to a
    // TempPath immediately: this closes our handle to the file, which
    // matters on Windows where an editor process cannot open a file that
    // this process still holds open.
    let tmp = tempfile::Builder::new()
        .suffix(".md")
        .tempfile()?
        .into_temp_path();
    let path = tmp.to_path_buf();

    let status = std::process::Command::new(&editor).arg(&path).status()?;

    if !status.success() {
        // Never delete text the user already typed: on a non-zero editor
        // exit, read back whatever is there and only discard it if it is
        // genuinely empty. `tmp` is intentionally NOT dropped-and-cleaned
        // here -- `.keep()` hands over ownership of the path so the file
        // survives past this function returning an error.
        let existing = std::fs::read_to_string(&path).unwrap_or_default();
        if existing.trim().is_empty() {
            return Err(format!("{editor} exited with {status}").into());
        }
        let kept_path = tmp.keep()?;
        return Err(format!(
            "{editor} exited with {status}; your note text was preserved at {}",
            kept_path.display()
        )
        .into());
    }
    let body = match std::fs::read_to_string(&path) {
        Ok(b) => b,
        Err(e) => {
            // Non-UTF-8 (or otherwise unreadable) content: keep the file
            // and tell the user exactly where it is rather than silently
            // losing it. Persist via `.keep()` since `tmp` would otherwise
            // delete the file when dropped.
            let kept_path = tmp.keep()?;
            return Err(format!(
                "could not read note from {} (invalid UTF-8): {e}; the file was left in place",
                kept_path.display()
            )
            .into());
        }
    };
    Ok(body.trim().to_string())
}

/// Run the CLI end to end.
///
/// # Exit codes
/// - `0` — success (note captured, list/search rendered, status changed,
///   rename applied, `--help`/`--version` printed, or an `adopt --undo` with
///   nothing to undo).
/// - `1` — not-found: `done`/`open` given an id with no matching note.
/// - `2` — usage error: a `parse` failure, an empty/whitespace capture (via
///   `add`/bare text/`new`), or `done`/`open` given a syntactically invalid
///   id.
pub fn run(args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
    let inv = match parse(args) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("{e}");
            return Ok(2);
        }
    };

    // --help/--version must work even if the DB is corrupt, so handle them
    // before opening the database.
    match inv {
        Invocation::Help => {
            println!("{HELP_TEXT}");
            return Ok(0);
        }
        Invocation::Version => {
            println!("noteit {}", env!("CARGO_PKG_VERSION"));
            return Ok(0);
        }
        // Plugin install/status is filesystem-only and must work even without a
        // usable notes DB, so it is handled before the database is opened.
        Invocation::Plugin(cmd) => {
            let mut out = std::io::stdout().lock();
            return Ok(crate::plugin::command::run(&cmd, &mut out)?);
        }
        _ => {}
    }

    // A corrupt DB or failed migration is a HARD failure: do not create a
    // second DB, do not skip the migration.
    let db = default_db_path()?;
    let mut store = Store::open(&db)?;
    let cwd = std::env::current_dir()?;
    let mut out = std::io::stdout().lock();
    run_core(inv, &mut store, &cwd, &mut out)
}

/// The post-setup dispatch logic, extracted from [`run`] so it can be driven
/// in-process against an already-open [`Store`], an arbitrary `cwd`, and an
/// output sink -- without touching the real DB, environment, or stdout.
/// Behavior must stay identical to what `run` did inline before this split.
///
/// Public so integration tests (a separate crate) can call it directly, but
/// not part of the CLI's stable public API -- it takes an already-parsed
/// `Invocation` and internal types, not command-line arguments.
#[doc(hidden)]
pub fn run_core(
    inv: Invocation,
    store: &mut Store,
    cwd: &Path,
    out: &mut dyn Write,
) -> Result<i32, Box<dyn std::error::Error>> {
    let resolved = resolve(store, cwd)?;
    if let Some(w) = &resolved.warning {
        notice(format_args!("warning: {w}"));
    }

    // Adoption is automatic but ANNOUNCED -- it moves data between scopes,
    // so a wrong fold must never be invisible. Skipped for `adopt --undo`:
    // running auto-adopt right before undoing it would be pointless work
    // and semantically confusing (adopt-then-undo in one invocation).
    if !matches!(inv, Invocation::Adopt { undo: true })
        && let Some(r) = adopt_if_needed(store, &resolved)?
    {
        notice(format_args!(
            "adopted {} notes from {} paths into {}",
            r.notes_moved, r.paths_folded, r.project
        ));
    }
    // No re-resolve needed: adopt_if_needed only folds OTHER path-contexts'
    // notes INTO this resolved context -- it never changes `resolved`'s own
    // id, root_path, or display_name. Re-resolving here would just cost an
    // extra project_id() git lookup and upsert_context round-trip on every
    // invocation for no behavioral difference.
    let ctx = &resolved.context;

    match inv {
        Invocation::Capture(body) => {
            if body.trim().is_empty() {
                eprintln!("empty note, nothing saved");
                return Ok(2);
            }
            let n = store.add_note(ctx.id, &resolved.subpath, &body)?;
            writeln!(
                out,
                "saved {} to {}",
                render::short_id(n.id),
                ctx.display_name
            )?;
        }
        Invocation::New => {
            let body = edit_in_editor()?;
            // "Nothing was saved" is not success: a caller must be able to
            // detect it consistently regardless of which path (Capture or
            // New) produced it, so this matches Capture's Ok(2) and message
            // -- the same convention `git commit` uses for an aborted
            // (empty-message) commit.
            if body.is_empty() {
                eprintln!("empty note, nothing saved");
                return Ok(2);
            }
            let n = store.add_note(ctx.id, &resolved.subpath, &body)?;
            writeln!(
                out,
                "saved {} to {}",
                render::short_id(n.id),
                ctx.display_name
            )?;
        }
        Invocation::List(a) => {
            let limit = effective_limit(a.limit);
            if let Some(tag) = a.tag {
                let scope = if a.global { None } else { Some(ctx.id) };
                let (all, total) = store.notes_by_tag(&tag, scope, a.all, None)?;
                let mut rows = all;
                // render_grouped requires rows sorted by context -- unsorted
                // input repeats a project's header. Sort before truncating so
                // the cap applies to what is actually shown.
                if a.global && !a.flat {
                    // display_name first for stable alphabetical grouping, then
                    // ctx.id BEFORE created_at so two distinct contexts that
                    // happen to share a display_name stay contiguous -- otherwise
                    // render_grouped (which requires contiguity by context) would
                    // interleave their rows and print duplicate-looking headers.
                    rows.sort_by(|x, y| {
                        x.0.display_name
                            .cmp(&y.0.display_name)
                            .then(x.0.id.cmp(&y.0.id))
                            .then(y.1.created_at.cmp(&x.1.created_at))
                    });
                }
                if let Some(n) = limit {
                    rows.truncate(n);
                }
                if a.global {
                    let s = if a.flat {
                        render::render_flat(&rows, total, limit)
                    } else {
                        render::render_grouped(&rows, total, limit)
                    };
                    writeln!(out, "{s}")?;
                } else {
                    let notes: Vec<_> = rows.into_iter().map(|(_, n)| n).collect();
                    writeln!(out, "{}", render::render_list(&notes, limit, total))?;
                }
            } else if a.global {
                let (all, total) = store.list_all_notes(a.all, None)?;
                let mut rows = all;
                if !a.flat {
                    // See the tag-grouped branch above: ctx.id must sit before
                    // created_at so same-named-but-distinct contexts stay
                    // contiguous for render_grouped.
                    rows.sort_by(|x, y| {
                        x.0.display_name
                            .cmp(&y.0.display_name)
                            .then(x.0.id.cmp(&y.0.id))
                            .then(y.1.created_at.cmp(&x.1.created_at))
                    });
                }
                if let Some(n) = limit {
                    rows.truncate(n);
                }
                let s = if a.flat {
                    render::render_flat(&rows, total, limit)
                } else {
                    render::render_grouped(&rows, total, limit)
                };
                writeln!(out, "{s}")?;
            } else {
                let (notes, total) = store.list_notes(ctx.id, None, a.all, limit)?;
                writeln!(out, "{}", render::render_list(&notes, limit, total))?;
            }
        }
        Invocation::Search { query, global } => {
            let scope = if global { None } else { Some(ctx.id) };
            let limit = effective_limit(None);
            let (rows, total) = store.search(&query, scope, limit)?;
            writeln!(out, "{}", render::render_flat(&rows, total, limit))?;
        }
        Invocation::SetStatus { id, status } => {
            let Some(rowid) = render::parse_short_id(&id) else {
                eprintln!("not a valid id: {id}");
                return Ok(2);
            };
            if store.set_status(rowid, status)? {
                writeln!(out, "{id} -> {}", status.as_str())?;
            } else {
                eprintln!("no note with id {id}");
                return Ok(1);
            }
        }
        Invocation::Rename(name) => {
            store.rename_context(ctx.id, &name)?;
            writeln!(out, "renamed to {name}")?;
        }
        Invocation::Adopt { undo: true } => match store.undo_last_adoption(ctx.id)? {
            Some(report) => {
                if report.notes_restored > 0 {
                    writeln!(
                        out,
                        "un-adopted {} notes to {} paths — now path-bound, view with: noteit list --global",
                        report.notes_restored, report.paths_restored
                    )?;
                } else {
                    writeln!(
                        out,
                        "un-adopted {} notes to {} paths",
                        report.notes_restored, report.paths_restored
                    )?;
                }
            }
            None => {
                writeln!(out, "nothing to undo")?;
            }
        },
        // Unreachable: parse() rejects bare `adopt` (no --undo) via
        // CliError::AdoptNeedsUndo before an Invocation is ever produced.
        Invocation::Adopt { undo: false } => {
            eprintln!("{}", CliError::AdoptNeedsUndo);
            return Ok(2);
        }
        Invocation::Delete { id } => {
            let Some(rowid) = render::parse_short_id(&id) else {
                eprintln!("not a valid id: {id}");
                return Ok(2);
            };
            match store.delete_note(rowid, ctx.id)? {
                Some(body) => {
                    let first_line = body.lines().next().unwrap_or("");
                    let truncated = first_line.chars().count() > 60;
                    let snippet: String = first_line.chars().take(60).collect();
                    let snippet = if truncated {
                        format!("{snippet}…")
                    } else {
                        snippet
                    };
                    writeln!(out, "deleted {id}: {snippet}")?;
                }
                None => {
                    eprintln!("no note with id {id}");
                    return Ok(1);
                }
            }
        }
        Invocation::Export => {
            // Full backup: every note in every context, including done ones.
            let (rows, _total) = store.list_all_notes(true, None)?;
            write!(out, "{}", render::export_json(&rows))?;
        }
        Invocation::Help | Invocation::Version => unreachable!("handled above"),
        // Plugin ops are dispatched in `run` before the DB is opened.
        Invocation::Plugin(_) => unreachable!("handled in run() before DB open"),
    }
    Ok(0)
}
