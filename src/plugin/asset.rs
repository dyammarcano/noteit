//! Portable plugin assets and their in-memory registry.
//!
//! Ported from an internal Go plugin-host package. An [`Asset`] is one plugin file
//! (frontmatter + body) classified by [`Kind`]; [`Asset::render`] emits on-disk
//! markdown bytes with a `created:` marker injected per asset-type convention
//! (skills → frontmatter, others → trailing HTML comment).

use std::error::Error;
use std::fmt;

/// Classifies a portable plugin asset so cross-host code can query the registry
/// without caring which host originally authored it.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Kind {
    /// Unclassified / zero value.
    #[default]
    Unknown,
    /// `commands/<name>.md` — slash command body + frontmatter.
    Command,
    /// `agents/<name>.md` — subagent definition.
    Agent,
    /// `skills/<name>/SKILL.md` — skill instructions.
    Skill,
}

impl Kind {
    /// Returns the lowercase identifier suitable for log lines.
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Command => "command",
            Kind::Agent => "agent",
            Kind::Skill => "skill",
            Kind::Unknown => "unknown",
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Template delimiters — chosen to avoid clashing with markdown `{{...}}`
/// examples in asset bodies.
pub const TEMPLATE_DELIMS_START: &str = "<%";
/// Closing template delimiter. See [`TEMPLATE_DELIMS_START`].
pub const TEMPLATE_DELIMS_END: &str = "%>";
/// Fallback `created:` date when neither the asset nor the render data set one.
pub const DEFAULT_CREATED: &str = "2026-05-24";

/// Feeds [`Asset::render`] at install time so author-side `<%.Name%>`
/// placeholders resolve to the host's published values.
#[derive(Clone, Default, Debug)]
pub struct TemplateData {
    /// Plugin name (`<%.Name%>`).
    pub name: String,
    /// Plugin version (`<%.Version%>`).
    pub version: String,
    /// Plugin description (`<%.Description%>`).
    pub description: String,
    /// MCP invocation command (`<%.McpCommand%>`).
    pub mcp_command: String,
    /// Creation date (`<%.Created%>`); defaults per [`Asset`] then
    /// [`DEFAULT_CREATED`].
    pub created: String,
}

/// One portable plugin file (frontmatter + body) classified by [`Kind`].
#[derive(Clone, Default, Debug)]
pub struct Asset {
    /// Asset classification.
    pub kind: Kind,
    /// Forward-slash path relative to the plugin root.
    pub path: String,
    /// YAML between `---` markers, or empty.
    pub frontmatter: String,
    /// Markdown body after the frontmatter.
    pub body: String,
    /// `YYYY-MM-DD`; falls back to [`DEFAULT_CREATED`].
    pub created: String,
}

/// Error returned by [`Asset::render`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenderError {
    /// A `<%` opened without a matching `%>`.
    UnterminatedDelimiter,
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenderError::UnterminatedDelimiter => {
                f.write_str("unterminated template delimiter: '<%' without matching '%>'")
            }
        }
    }
}

impl Error for RenderError {}

impl Asset {
    fn created_or_default(&self) -> String {
        if self.created.is_empty() {
            DEFAULT_CREATED.to_string()
        } else {
            self.created.clone()
        }
    }

    /// Produces on-disk bytes. The caller supplies [`TemplateData`] so
    /// host-specific names/versions flow into the published file.
    pub fn render(&self, mut d: TemplateData) -> Result<Vec<u8>, RenderError> {
        if d.created.is_empty() {
            d.created = self.created_or_default();
        }

        let mut out = String::new();
        if !self.frontmatter.is_empty() {
            out.push_str("---\n");
            out.push_str(&substitute(&self.frontmatter, &d)?);
            if self.kind == Kind::Skill {
                let fm = &self.frontmatter;
                if !fm.contains("\ncreated:") && !fm.starts_with("created:") {
                    out.push_str(&format!("created: {}\n", d.created));
                }
            }
            out.push_str("---\n");
        }
        out.push_str(&substitute(&self.body, &d)?);
        if self.kind != Kind::Skill {
            if !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str(&format!("\n<!-- created:{} -->\n", d.created));
        }
        Ok(out.into_bytes())
    }
}

/// Resolves `<%.Field%>` placeholders in `src` against `d`. Unknown fields are
/// left untouched (passthrough); an unterminated `<%` is an error.
fn substitute(src: &str, d: &TemplateData) -> Result<String, RenderError> {
    let mut out = String::with_capacity(src.len());
    let mut rest = src;
    while let Some(start) = rest.find(TEMPLATE_DELIMS_START) {
        out.push_str(&rest[..start]);
        let after = &rest[start + TEMPLATE_DELIMS_START.len()..];
        let Some(end) = after.find(TEMPLATE_DELIMS_END) else {
            return Err(RenderError::UnterminatedDelimiter);
        };
        let expr = after[..end].trim();
        match field_value(expr, d) {
            Some(v) => out.push_str(v),
            None => {
                out.push_str(TEMPLATE_DELIMS_START);
                out.push_str(&after[..end]);
                out.push_str(TEMPLATE_DELIMS_END);
            }
        }
        rest = &after[end + TEMPLATE_DELIMS_END.len()..];
    }
    out.push_str(rest);
    Ok(out)
}

fn field_value<'a>(expr: &str, d: &'a TemplateData) -> Option<&'a str> {
    match expr {
        ".Name" => Some(&d.name),
        ".Version" => Some(&d.version),
        ".Description" => Some(&d.description),
        ".McpCommand" => Some(&d.mcp_command),
        ".Created" => Some(&d.created),
        _ => None,
    }
}

/// In-memory registry of every plugin asset a host contributes. Replaces Go's
/// global `assetRegistry` populated via `init()` with an explicit owned value.
#[derive(Clone, Default, Debug)]
pub struct AssetRegistry {
    assets: Vec<Asset>,
}

impl AssetRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds assets to the registry.
    pub fn register<I>(&mut self, assets: I)
    where
        I: IntoIterator<Item = Asset>,
    {
        self.assets.extend(assets);
    }

    /// Returns the asset matching `(kind, path)`, if any.
    pub fn by_path(&self, kind: Kind, path: &str) -> Option<Asset> {
        self.assets
            .iter()
            .find(|a| a.kind == kind && a.path == path)
            .cloned()
    }

    /// Returns every asset of the given kind, sorted by path.
    pub fn by_kind(&self, kind: Kind) -> Vec<Asset> {
        let mut out: Vec<Asset> = self
            .assets
            .iter()
            .filter(|a| a.kind == kind)
            .cloned()
            .collect();
        out.sort_by(|a, b| a.path.cmp(&b.path));
        out
    }

    /// Returns every registered asset, sorted by path.
    pub fn all(&self) -> Vec<Asset> {
        let mut out = self.assets.clone();
        out.sort_by(|a, b| a.path.cmp(&b.path));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ported from asset_test.go: TestRender_FrontmatterAndBodyTemplating.
    #[test]
    fn render_frontmatter_and_body_templating() {
        let a = Asset {
            kind: Kind::Command,
            path: "commands/example.md".into(),
            frontmatter: "description: <%.Description%>\n".into(),
            body: "Run `<%.McpCommand%> mcp serve`.\n".into(),
            created: "2026-05-24".into(),
        };
        let got = a
            .render(TemplateData {
                description: "hello".into(),
                mcp_command: "noteit".into(),
                ..Default::default()
            })
            .expect("render");
        let s = String::from_utf8(got).unwrap();
        assert!(s.contains("description: hello"), "frontmatter subst: {s:?}");
        assert!(s.contains("Run `noteit mcp serve`"), "body subst: {s:?}");
        assert!(
            s.contains("<!-- created:2026-05-24 -->"),
            "created marker: {s:?}"
        );
    }

    // Ported from asset_test.go: TestRender_SkillEmitsCreatedInFrontmatter.
    #[test]
    fn render_skill_emits_created_in_frontmatter() {
        let a = Asset {
            kind: Kind::Skill,
            path: "skills/x/SKILL.md".into(),
            frontmatter: "name: x\n".into(),
            body: "body\n".into(),
            created: "2026-05-24".into(),
        };
        let s = String::from_utf8(a.render(TemplateData::default()).expect("render")).unwrap();
        assert!(
            s.contains("created: 2026-05-24"),
            "skill frontmatter created: {s:?}"
        );
        assert!(
            !s.contains("<!-- created:"),
            "skill must not use trailing marker: {s:?}"
        );
    }

    // Ported from asset_test.go: TestRender_FallsBackToDefaultCreated.
    #[test]
    fn render_falls_back_to_default_created() {
        let a = Asset {
            kind: Kind::Command,
            path: "commands/x.md".into(),
            body: "x\n".into(),
            ..Default::default()
        };
        let s = String::from_utf8(a.render(TemplateData::default()).expect("render")).unwrap();
        assert!(
            s.contains(&format!("<!-- created:{DEFAULT_CREATED} -->")),
            "expected fallback DefaultCreated marker: {s:?}"
        );
    }

    // Ported from asset_test.go: TestKindString.
    #[test]
    fn kind_string() {
        assert_eq!(Kind::Unknown.as_str(), "unknown");
        assert_eq!(Kind::Command.as_str(), "command");
        assert_eq!(Kind::Agent.as_str(), "agent");
        assert_eq!(Kind::Skill.as_str(), "skill");
    }

    // Ported from registry_test.go: TestAssetByPathAndAssetsByKind.
    #[test]
    fn asset_by_path_and_assets_by_kind() {
        let mut reg = AssetRegistry::new();
        reg.register([
            Asset {
                kind: Kind::Skill,
                path: "skills/a/SKILL.md".into(),
                ..Default::default()
            },
            Asset {
                kind: Kind::Command,
                path: "commands/b.md".into(),
                ..Default::default()
            },
            Asset {
                kind: Kind::Command,
                path: "commands/a.md".into(),
                ..Default::default()
            },
        ]);

        assert!(reg.by_path(Kind::Skill, "skills/a/SKILL.md").is_some());
        assert!(
            reg.by_path(Kind::Command, "skills/a/SKILL.md").is_none(),
            "wrong kind"
        );

        let cmds = reg.by_kind(Kind::Command);
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].path, "commands/a.md", "sorted");
        assert_eq!(cmds[1].path, "commands/b.md", "sorted");

        assert_eq!(reg.all().len(), 3);
    }

    // Renderer-specific: unknown field is passed through untouched.
    #[test]
    fn render_unknown_field_passthrough() {
        let a = Asset {
            kind: Kind::Command,
            path: "commands/x.md".into(),
            body: "keep <%.Nope%> here\n".into(),
            ..Default::default()
        };
        let s = String::from_utf8(a.render(TemplateData::default()).expect("render")).unwrap();
        assert!(s.contains("keep <%.Nope%> here"), "passthrough: {s:?}");
    }

    // Renderer-specific: unterminated delimiter errors.
    #[test]
    fn render_unterminated_delimiter_errors() {
        let a = Asset {
            kind: Kind::Command,
            path: "commands/x.md".into(),
            body: "oops <%.Name".into(),
            ..Default::default()
        };
        assert_eq!(
            a.render(TemplateData::default()),
            Err(RenderError::UnterminatedDelimiter)
        );
    }
}
