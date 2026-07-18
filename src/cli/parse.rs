use crate::store::notes::Status;

/// The closed set of verbs. A first argument matching one of these
/// dispatches that verb; ANYTHING else is note text. The set is small and
/// known at parse time, which is what makes the rule unambiguous despite
/// looking magical. `add` is the escape hatch for colliding text.
pub const VERBS: &[&str] = &[
    "add", "list", "search", "new", "done", "open", "project", "adopt", "delete", "plugin",
    "export",
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
    #[error("{0}")]
    Plugin(#[from] crate::plugin::command::ParseError),
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
    Export,
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
    noteit export              dump all notes as JSON (backup)
    noteit project rename <n>  rename the current project
    noteit plugin install --host <claude|codex|gemini|all>
                               install noteit's assets into an AI host
    noteit plugin list | status | doctor | uninstall --host <h>
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
        "plugin" => Ok(Invocation::Plugin(crate::plugin::command::parse(rest)?)),
        "export" => Ok(Invocation::Export),
        _ => unreachable!("VERBS and match arms must stay in sync"),
    }
}
