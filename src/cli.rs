use crate::store::notes::Status;

/// The closed set of verbs. A first argument matching one of these
/// dispatches that verb; ANYTHING else is note text. The set is small and
/// known at parse time, which is what makes the rule unambiguous despite
/// looking magical. `add` is the escape hatch for colliding text.
pub const VERBS: &[&str] = &["add", "list", "search", "new", "done", "open", "project"];

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
    noteit project rename <n>  rename the current project
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
            Ok(Invocation::Search { query: query.join(" "), global })
        }
        "done" | "open" => {
            let id = rest.first().ok_or(CliError::StatusNeedsId(
                if first == "done" { "done" } else { "open" },
            ))?;
            let status = if first == "done" { Status::Done } else { Status::Open };
            Ok(Invocation::SetStatus { id: id.clone(), status })
        }
        "project" => {
            if rest.first().map(String::as_str) != Some("rename") || rest.len() < 2 {
                return Err(CliError::BadProject);
            }
            Ok(Invocation::Rename(rest[1..].join(" ")))
        }
        _ => unreachable!("VERBS and match arms must stay in sync"),
    }
}

use std::io::Write;

use crate::context::{adopt_if_needed, resolve};
use crate::render;
use crate::store::{default_db_path, Store};

const DEFAULT_LIMIT: usize = 50;

fn effective_limit(requested: Option<usize>) -> Option<usize> {
    match requested {
        Some(0) => None,        // --limit 0 means everything
        Some(n) => Some(n),
        None => Some(DEFAULT_LIMIT),
    }
}

/// Open a note body in $EDITOR (or $VISUAL) via a temp file.
fn edit_in_editor() -> Result<String, Box<dyn std::error::Error>> {
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| {
            if cfg!(windows) { "notepad".to_string() } else { "vi".to_string() }
        });
    let dir = std::env::temp_dir();
    let path = dir.join(format!("noteit-{}.md", std::process::id()));
    std::fs::write(&path, "")?;

    let status = match std::process::Command::new(&editor).arg(&path).status() {
        Ok(s) => s,
        Err(e) => {
            let _ = std::fs::remove_file(&path);
            return Err(e.into());
        }
    };
    if !status.success() {
        let _ = std::fs::remove_file(&path);
        return Err(format!("{editor} exited with {status}").into());
    }
    let body = std::fs::read_to_string(&path)?;
    let _ = std::fs::remove_file(&path);
    Ok(body.trim().to_string())
}

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
        _ => {}
    }

    // A corrupt DB or failed migration is a HARD failure: do not create a
    // second DB, do not skip the migration.
    let db = default_db_path()?;
    let mut store = Store::open(&db)?;

    let cwd = std::env::current_dir()?;
    let resolved = resolve(&store, &cwd)?;
    if let Some(w) = &resolved.warning {
        eprintln!("warning: {w}");
    }

    // Adoption is automatic but ANNOUNCED -- it moves data between scopes,
    // so a wrong fold must never be invisible.
    if let Some(r) = adopt_if_needed(&mut store, &resolved)? {
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

    let mut out = std::io::stdout().lock();
    match inv {
        Invocation::Capture(body) => {
            if body.trim().is_empty() {
                eprintln!("empty note, nothing saved");
                return Ok(2);
            }
            let n = store.add_note(ctx.id, &resolved.subpath, &body)?;
            writeln!(out, "saved {} to {}", render::short_id(n.id), ctx.display_name)?;
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
            writeln!(out, "saved {} to {}", render::short_id(n.id), ctx.display_name)?;
        }
        Invocation::List(a) => {
            let limit = effective_limit(a.limit);
            if a.global {
                let all = store.list_all_notes(a.all, None)?;
                let total = all.len();
                let mut rows = all;
                if !a.flat {
                    rows.sort_by(|x, y| {
                        x.0.display_name
                            .cmp(&y.0.display_name)
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
            } else if let Some(tag) = a.tag {
                let rows = store.notes_by_tag(&tag, Some(ctx.id))?;
                let total = rows.len();
                let notes: Vec<_> = rows.into_iter().map(|(_, n)| n).collect();
                writeln!(out, "{}", render::render_list(&notes, limit, total))?;
            } else {
                let total = store.list_notes(ctx.id, None, a.all, None)?.len();
                let notes = store.list_notes(ctx.id, None, a.all, limit)?;
                writeln!(out, "{}", render::render_list(&notes, limit, total))?;
            }
        }
        Invocation::Search { query, global } => {
            let scope = if global { None } else { Some(ctx.id) };
            let limit = effective_limit(None);
            let total = store.search(&query, scope, None)?.len();
            let rows = store.search(&query, scope, limit)?;
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
        Invocation::Help | Invocation::Version => unreachable!("handled above"),
    }
    Ok(0)
}
