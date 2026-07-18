//! noteit's own plugin assets and the [`TemplateData`] describing this build.
//!
//! These are the commands / agent / skill that noteit ships so an AI host
//! (Claude, Codex, Gemini) can drive the `noteit` CLI. Asset bodies use
//! `<%.McpCommand%>` for the binary name so a rename flows through at render.

use super::asset::{Asset, AssetRegistry, Kind, TemplateData};

/// Creation stamp for noteit's shipped assets.
const CREATED: &str = "2026-07-17";

/// The render data describing this noteit build.
pub fn template_data() -> TemplateData {
    TemplateData {
        name: "noteit".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        description: "Notes bound to the git repository you're in.".to_string(),
        mcp_command: "noteit".to_string(),
        created: CREATED.to_string(),
    }
}

/// Builds the registry of noteit's own plugin assets.
pub fn registry() -> AssetRegistry {
    let mut reg = AssetRegistry::new();
    reg.register([
        skill(),
        cmd_note(),
        cmd_notes(),
        cmd_note_search(),
        agent_note_keeper(),
    ]);
    reg
}

fn skill() -> Asset {
    Asset {
        kind: Kind::Skill,
        path: "skills/noteit/SKILL.md".to_string(),
        frontmatter: "name: noteit\n\
description: Capture and recall notes bound to the current git repository using the noteit CLI.\n"
            .to_string(),
        body: "\
# noteit

`<%.McpCommand%>` captures notes bound to the git repository you're in. Notes
follow a repo across clones and renames (they key off the repo's first commit),
so recall is scoped to *this* project automatically.

## When to use

- The user wants to jot an idea, TODO, or decision tied to the current repo.
- The user asks \"what notes do I have here?\" or wants to search past notes.

## How to use

- Capture: `<%.McpCommand%> add \"<text>\"` (or `<%.McpCommand%> <text>` when the
  text doesn't collide with a subcommand).
- List for this repo: `<%.McpCommand%> list` (add `--global` for every repo).
- Search: `<%.McpCommand%> search \"<query>\"` (full-text; `--global` to widen).
- Mark done / reopen: `<%.McpCommand%> done <id>` / `<%.McpCommand%> open <id>`.

Run the command with the host's shell/exec tool from the repository directory so
the note binds to the right context. Notes print to stdout; adoption and
warnings go to stderr.
"
        .to_string(),
        created: CREATED.to_string(),
    }
}

fn cmd_note() -> Asset {
    Asset {
        kind: Kind::Command,
        path: "commands/note.md".to_string(),
        frontmatter: "description: Capture a note bound to the current git repository.\n"
            .to_string(),
        body: "\
Capture the user's note in the current repository with noteit.

Run: `<%.McpCommand%> add \"$ARGUMENTS\"` from the repository directory, then
confirm what was saved (noteit prints `saved <id> to <project>`).
"
        .to_string(),
        created: CREATED.to_string(),
    }
}

fn cmd_notes() -> Asset {
    Asset {
        kind: Kind::Command,
        path: "commands/notes.md".to_string(),
        frontmatter: "description: List notes for the current git repository.\n".to_string(),
        body: "\
List the notes bound to the current repository.

Run: `<%.McpCommand%> list` from the repository directory (add `--global` to list
notes across every repository, `--all` to include notes marked done). Summarize
the results for the user.
"
        .to_string(),
        created: CREATED.to_string(),
    }
}

fn cmd_note_search() -> Asset {
    Asset {
        kind: Kind::Command,
        path: "commands/note-search.md".to_string(),
        frontmatter: "description: Full-text search notes in the current repository.\n".to_string(),
        body: "\
Search the user's notes.

Run: `<%.McpCommand%> search \"$ARGUMENTS\"` from the repository directory (add
`--global` to search every repository). Present the matching notes.
"
        .to_string(),
        created: CREATED.to_string(),
    }
}

fn agent_note_keeper() -> Asset {
    Asset {
        kind: Kind::Agent,
        path: "agents/note-keeper.md".to_string(),
        frontmatter: "name: note-keeper\n\
description: Manages repo-bound notes via the noteit CLI — captures, lists, searches, and curates.\n"
            .to_string(),
        body: "\
You manage the user's repository-bound notes with the `<%.McpCommand%>` CLI.

Capabilities:
- Capture: `<%.McpCommand%> add \"<text>\"`
- List: `<%.McpCommand%> list` (`--global`, `--all`, `--tag <t>`)
- Search: `<%.McpCommand%> search \"<query>\"` (`--global`)
- Curate: `<%.McpCommand%> done <id>` / `open <id>` / `delete <id>`

Always run from the repository directory so notes bind to the right context.
Never delete a note unless the user explicitly asks. Report exactly what the CLI
printed; do not invent note ids.
"
        .to_string(),
        created: CREATED.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_all_asset_kinds() {
        let reg = registry();
        assert_eq!(reg.by_kind(Kind::Skill).len(), 1);
        assert_eq!(reg.by_kind(Kind::Command).len(), 3);
        assert_eq!(reg.by_kind(Kind::Agent).len(), 1);
    }

    #[test]
    fn assets_render_with_binary_name_substituted() {
        let reg = registry();
        let data = template_data();
        for a in reg.all() {
            let bytes = a.render(data.clone()).expect("render");
            let s = String::from_utf8(bytes).unwrap();
            assert!(
                !s.contains("<%."),
                "unresolved placeholder in {}: {s:?}",
                a.path
            );
            assert!(s.contains("noteit"), "binary name missing in {}", a.path);
        }
    }

    #[test]
    fn version_matches_crate() {
        assert_eq!(template_data().version, env!("CARGO_PKG_VERSION"));
    }
}
