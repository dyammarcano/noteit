use crate::store::notes::Status;

/// The closed set of verbs. A first argument matching one of these
/// dispatches that verb; ANYTHING else is note text. The set is small and
/// known at parse time, which is what makes the rule unambiguous despite
/// looking magical. `add` is the escape hatch for colliding text.
pub const VERBS: &[&str] = &[
    "add", "list", "search", "new", "done", "open", "project", "adopt", "delete", "plugin",
];

#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("usage: noteit add <text>")]
    AddNeedsText,
    #[error("usage: noteit search <query>")]
    SearchNeedsQuery,
    #[error("usage: noteit {0} <id>")]
    StatusNeedsId(&'static str),
    #[error("usage: noteit project rename <name>")]
    BadProject,
    #[error("--limit needs a number")]
    BadLimit,
    #[error("usage: noteit list --tag <name>")]
    TagNeedsValue,
    #[error("unknown flag: {0}")]
    UnknownFlag(String),
    #[error(
        "usage: noteit adopt --undo  (adoption is automatic; --undo reverses the most recent one)"
    )]
    AdoptNeedsUndo,
    #[error("usage: noteit delete <id>")]
    DeleteNeedsId,
    #[error("usage: noteit plugin install|uninstall --host <claude|codex|gemini|all>")]
    PluginNeedsHost,
    #[error("unknown plugin subcommand: {0} (try: list, install, status, uninstall)")]
    PluginUnknownSub(String),
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ListArgs {
    pub global: bool,
    pub flat: bool,
    pub tag: Option<String>,
    pub all: bool,
    pub limit: Option<usize>,
}

#[derive(Debug)]
pub enum Invocation {
    Capture(String),
    List(ListArgs),
    Search { query: String, global: bool },
    New,
    SetStatus { id: String, status: Status },
    Rename(String),
    Help,
    Version,
    Adopt { undo: bool },
    Delete { id: String },
    Plugin(crate::plugin::PluginCmd),
}

pub const HELP_TEXT: &str = "\
noteit — notes bound to the directory you're in

USAGE:
    noteit <text>              capture a note in the current context
    noteit                     list notes for the current context
    noteit add <text>          capture text that collides with a verb
    noteit new                 capture a longer note in $EDITOR
    noteit search <query>      full-text search      [--global]
    noteit list                list notes            [--global] [--flat] [--tag <t>] [--all] [--limit <n>]
    noteit done <id>           mark a note done
    noteit open <id>           reopen a note
    noteit delete <id>   delete a note permanently
    noteit project rename <n>  rename the current project
    noteit plugin install --host <claude|codex|gemini|all>
                               install noteit's assets into an AI host
    noteit plugin list | status | uninstall --host <h>
    noteit --help | --version

NOTES:
    A first argument matching a known verb runs that verb; anything else is
    note text. `noteit search this` searches — use `noteit add \"search this\"`
    to capture that text instead.

    Notes bind to a repo's identity (derived from its first commit), so they
    follow the repo across clones and renames. Outside a repo, notes bind to
    the directory path, and are adopted into the repo if one appears later.";

fn parse_list_args(rest: &[String]) -> Result<ListArgs, CliError> {
    let mut a = ListArgs::default();
    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "--global" | "-g" => a.global = true,
            "--flat" => a.flat = true,
            "--all" => a.all = true,
            "--tag" => {
                i += 1;
                a.tag = Some(rest.get(i).cloned().ok_or(CliError::TagNeedsValue)?);
            }
            "--limit" => {
                i += 1;
                let v = rest.get(i).ok_or(CliError::BadLimit)?;
                a.limit = Some(v.parse().map_err(|_| CliError::BadLimit)?);
            }
            other => return Err(CliError::UnknownFlag(other.to_string())),
        }
        i += 1;
    }
    Ok(a)
}

fn parse_host_flag(rest: &[String]) -> Result<Option<crate::plugin::HostSel>, CliError> {
    use crate::plugin::HostSel;
    let mut sel = None;
    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "--host" => {
                i += 1;
                let v = rest.get(i).ok_or(CliError::PluginNeedsHost)?;
                sel = Some(if v == "all" {
                    HostSel::All
                } else {
                    HostSel::One(v.clone())
                });
            }
            other => return Err(CliError::UnknownFlag(other.to_string())),
        }
        i += 1;
    }
    Ok(sel)
}

fn parse_plugin(rest: &[String]) -> Result<crate::plugin::PluginCmd, CliError> {
    use crate::plugin::PluginCmd;
    let (sub, flags) = match rest.split_first() {
        Some((s, f)) => (s.as_str(), f),
        None => ("list", &[][..]),
    };
    match sub {
        "list" => Ok(PluginCmd::List),
        "install" => Ok(PluginCmd::Install(
            parse_host_flag(flags)?.ok_or(CliError::PluginNeedsHost)?,
        )),
        "uninstall" => Ok(PluginCmd::Uninstall(
            parse_host_flag(flags)?.ok_or(CliError::PluginNeedsHost)?,
        )),
        "status" => Ok(PluginCmd::Status(parse_host_flag(flags)?)),
        other => Err(CliError::PluginUnknownSub(other.to_string())),
    }
}

pub fn parse(args: &[String]) -> Result<Invocation, CliError> {
    let Some(first) = args.first() else {
        return Ok(Invocation::List(ListArgs::default()));
    };

    // --help/--version must never be captured as note text, so they are
    // checked before the VERBS ambiguity rule kicks in.
    match first.as_str() {
        "--help" | "-h" => return Ok(Invocation::Help),
        "--version" | "-V" => return Ok(Invocation::Version),
        _ => {}
    }

    let rest = &args[1..];

    if !VERBS.contains(&first.as_str()) {
        return Ok(Invocation::Capture(args.join(" ")));
    }

    match first.as_str() {
        "add" => {
            if rest.is_empty() {
                return Err(CliError::AddNeedsText);
            }
            Ok(Invocation::Capture(rest.join(" ")))
        }
        "list" => Ok(Invocation::List(parse_list_args(rest)?)),
        "new" => Ok(Invocation::New),
        "search" => {
            // Note: this filters every --global/-g token out of the query, so a
            // literal search for the string "--global" is not possible in v1 --
            // the flag always wins.
            let global = rest.iter().any(|a| a == "--global" || a == "-g");
            let query: Vec<&str> = rest
                .iter()
                .filter(|a| *a != "--global" && *a != "-g")
                .map(|s| s.as_str())
                .collect();
            if query.is_empty() {
                return Err(CliError::SearchNeedsQuery);
            }
            Ok(Invocation::Search {
                query: query.join(" "),
                global,
            })
        }
        "done" | "open" => {
            let id = rest
                .first()
                .ok_or(CliError::StatusNeedsId(if first == "done" {
                    "done"
                } else {
                    "open"
                }))?;
            let status = if first == "done" {
                Status::Done
            } else {
                Status::Open
            };
            Ok(Invocation::SetStatus {
                id: id.clone(),
                status,
            })
        }
        "project" => {
            if rest.first().map(String::as_str) != Some("rename") || rest.len() < 2 {
                return Err(CliError::BadProject);
            }
            Ok(Invocation::Rename(rest[1..].join(" ")))
        }
        "adopt" => {
            if rest.iter().any(|a| a == "--undo") {
                Ok(Invocation::Adopt { undo: true })
            } else {
                Err(CliError::AdoptNeedsUndo)
            }
        }
        "delete" => {
            let id = rest.first().ok_or(CliError::DeleteNeedsId)?;
            Ok(Invocation::Delete { id: id.clone() })
        }
        "plugin" => Ok(Invocation::Plugin(parse_plugin(rest)?)),
        _ => unreachable!("VERBS and match arms must stay in sync"),
    }
}

use std::io::Write;
use std::path::Path;

use crate::context::{adopt_if_needed, resolve};
use crate::render;
use crate::store::{Store, default_db_path};

const DEFAULT_LIMIT: usize = 50;

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
        eprintln!("warning: {w}");
    }

    // Adoption is automatic but ANNOUNCED -- it moves data between scopes,
    // so a wrong fold must never be invisible. Skipped for `adopt --undo`:
    // running auto-adopt right before undoing it would be pointless work
    // and semantically confusing (adopt-then-undo in one invocation).
    if !matches!(inv, Invocation::Adopt { undo: true })
        && let Some(r) = adopt_if_needed(store, &resolved)?
    {
        eprintln!(
            "adopted {} notes from {} paths into {}",
            r.notes_moved, r.paths_folded, r.project
        );
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
        Invocation::Help | Invocation::Version => unreachable!("handled above"),
        // Plugin ops are dispatched in `run` before the DB is opened.
        Invocation::Plugin(_) => unreachable!("handled in run() before DB open"),
    }
    Ok(0)
}
