//! Synthesized library skills that preserve command/agent discovery on hosts
//! whose install surface is skills-only.
//!
//! Ported from an internal Go plugin-host package and **de-identified**: every
//! source-project-specific literal is replaced with a neutral, host-agnostic
//! equivalent so this module is safe to ship in the public noteit repo. The
//! guard test below enforces that no such literal survives into rendered output.

use super::asset::{Asset, AssetRegistry, Kind};
use std::fmt::Write as _;
use std::path::Path;

/// Install path of the synthesized command-library skill.
pub const COMMAND_LIBRARY_SKILL_PATH: &str = "skills/command-library/SKILL.md";
/// Install path of the synthesized agent-library skill.
pub const AGENT_LIBRARY_SKILL_PATH: &str = "skills/agent-library/SKILL.md";

/// Returns synthesized skills that preserve command and agent discovery for
/// hosts whose install surface is skills-only. Reads the registered command and
/// agent assets from `reg` to build the index.
pub fn portable_library_skills(reg: &AssetRegistry) -> Vec<Asset> {
    vec![
        library_skill(
            reg,
            Kind::Command,
            COMMAND_LIBRARY_SKILL_PATH,
            "command-library",
            "Use bundled slash-command prompts as portable workflow references.",
            "Command Library",
            "Use these entries when the user asks for a slash-command style workflow. \
Read the matching command asset by name, adapt its instructions to the current host, \
and execute the workflow with the active tools available in this session.",
        ),
        library_skill(
            reg,
            Kind::Agent,
            AGENT_LIBRARY_SKILL_PATH,
            "agent-library",
            "Use bundled agent prompts as portable subagent-style references.",
            "Agent Library",
            "Use these entries when the task benefits from a specialist role. \
Read the matching agent prompt by name, apply its role, inputs, workflow, output contract, \
and safety rules in the current host, and adapt host-specific tool names where needed.",
        ),
    ]
}

fn library_skill(
    reg: &AssetRegistry,
    kind: Kind,
    asset_path: &str,
    skill_name: &str,
    description: &str,
    title: &str,
    guidance: &str,
) -> Asset {
    let assets = reg.by_kind(kind); // already sorted by path

    let mut b = String::new();
    let _ = write!(b, "# {title}\n\n{guidance}\n\n");
    b.push_str("## Index\n\n");
    for a in &assets {
        let name = Path::new(&a.path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&a.path);
        let mut desc = frontmatter_description(&a.frontmatter);
        if desc.is_empty() {
            desc = "No description provided.".to_string();
        }
        let _ = writeln!(b, "- `{name}` (`{}`) - {desc}", a.path);
    }
    if assets.is_empty() {
        b.push_str("- No assets are registered for this library.\n");
    }
    b.push_str("\n## Adaptation Rules\n\n");
    b.push_str(
        "- Keep the source prompt's behavioral contract, but map host-specific command, \
agent, and tool syntax onto this host's available tools.\n",
    );
    b.push_str("- Prefer the active MCP server for tool calls when available.\n");
    b.push_str(
        "- If a source prompt references a bundled workflow, reference, or template path, \
treat it as bundled prompt material and load only the parts needed for the current task.\n",
    );

    Asset {
        kind: Kind::Skill,
        path: asset_path.to_string(),
        frontmatter: format!("name: {skill_name}\ndescription: {description}\n"),
        body: b,
        created: String::new(),
    }
}

/// Extracts the `description:` value from a frontmatter block, stripping quotes.
fn frontmatter_description(fm: &str) -> String {
    for line in fm.split('\n') {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("description:") {
            return rest.trim().trim_matches(['"', '\'']).to_string();
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry_with_commands() -> AssetRegistry {
        let mut reg = AssetRegistry::new();
        reg.register([
            Asset {
                kind: Kind::Command,
                path: "commands/beta.md".into(),
                frontmatter: "description: The beta command.\n".into(),
                ..Default::default()
            },
            Asset {
                kind: Kind::Command,
                path: "commands/alpha.md".into(),
                frontmatter: "description: \"The alpha command.\"\n".into(),
                ..Default::default()
            },
            Asset {
                kind: Kind::Agent,
                path: "agents/scout.md".into(),
                frontmatter: "name: scout\n".into(), // no description
                ..Default::default()
            },
        ]);
        reg
    }

    #[test]
    fn builds_sorted_indexed_command_library() {
        let reg = registry_with_commands();
        let skills = portable_library_skills(&reg);
        assert_eq!(skills.len(), 2);

        let cmd = &skills[0];
        assert_eq!(cmd.kind, Kind::Skill);
        assert_eq!(cmd.path, COMMAND_LIBRARY_SKILL_PATH);
        assert!(cmd.frontmatter.contains("name: command-library"));
        // Sorted by path: alpha before beta.
        let alpha = cmd.body.find("`alpha`").unwrap();
        let beta = cmd.body.find("`beta`").unwrap();
        assert!(alpha < beta, "index sorted by path");
        assert!(cmd.body.contains("The alpha command."), "quotes stripped");
        assert!(cmd.body.contains("The beta command."));
    }

    #[test]
    fn missing_description_falls_back() {
        let reg = registry_with_commands();
        let agent = &portable_library_skills(&reg)[1];
        assert_eq!(agent.path, AGENT_LIBRARY_SKILL_PATH);
        assert!(agent.body.contains("No description provided."));
    }

    #[test]
    fn empty_registry_notes_no_assets() {
        let reg = AssetRegistry::new();
        let skills = portable_library_skills(&reg);
        assert!(
            skills[0]
                .body
                .contains("No assets are registered for this library.")
        );
    }

    // Public-repo guard: the private source project's name (and other
    // source-specific phrasing) must never leak into shipped, rendered output.
    // The forbidden tokens below are the enforcement mechanism, not provenance.
    #[test]
    fn de_identified_no_private_literals() {
        let forbidden = ["lensr", "claude-only", "inovacc"];
        let reg = registry_with_commands();
        for s in portable_library_skills(&reg) {
            let hay = format!("{}\n{}\n{}", s.path, s.frontmatter, s.body).to_lowercase();
            for token in forbidden {
                assert!(
                    !hay.contains(token),
                    "private literal {token:?} leaked: {}",
                    s.path
                );
            }
        }
    }
}
