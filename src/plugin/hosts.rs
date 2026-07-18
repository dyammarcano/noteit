//! Concrete plugin hosts noteit can install into: Claude, Codex, Gemini.
//!
//! Each [`NoteitHost`] renders noteit's shared asset tree ([`crate::plugin::noteit`])
//! into a host-specific directory under the user's home. Claude uses its real
//! `.claude-plugin/plugin.json` manifest and native `commands/`/`agents/`;
//! skills-forward hosts (Codex, Gemini) additionally receive the synthesized
//! library skills so command/agent discovery survives a skills-only surface.

use std::collections::BTreeMap;
use std::io;
use std::path::PathBuf;

use super::asset::{AssetRegistry, TemplateData};
use super::host::Host;
use super::libraries::portable_library_skills;
use super::write_tree::{TreeWriter, write_tree_atomic};
use super::{Installer, noteit};

/// Where a host writes its plugin manifest.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ManifestStyle {
    /// `.claude-plugin/plugin.json` (Claude Code's native layout).
    ClaudePlugin,
    /// `plugin.json` at the plugin root.
    Generic,
}

/// A concrete plugin host target for noteit.
pub struct NoteitHost {
    name: String,
    /// Home-relative host config dir, e.g. `.claude`.
    dir_name: String,
    manifest: ManifestStyle,
    include_library_skills: bool,
    assets: AssetRegistry,
    data: TemplateData,
}

/// The directories a host install sweeps for stale assets.
pub const SWEEP_DIRS: &[&str] = &["commands", "agents", "skills"];

impl NoteitHost {
    fn new(
        name: &str,
        dir_name: &str,
        manifest: ManifestStyle,
        include_library_skills: bool,
    ) -> Self {
        Self {
            name: name.to_string(),
            dir_name: dir_name.to_string(),
            manifest,
            include_library_skills,
            assets: noteit::registry(),
            data: noteit::template_data(),
        }
    }

    /// The Claude Code host (native commands + agents + skills).
    pub fn claude() -> Self {
        Self::new("claude", ".claude", ManifestStyle::ClaudePlugin, false)
    }

    /// The Codex host (skills-forward, generic manifest).
    pub fn codex() -> Self {
        Self::new("codex", ".codex", ManifestStyle::Generic, true)
    }

    /// The Gemini host (skills-forward, generic manifest).
    pub fn gemini() -> Self {
        Self::new("gemini", ".gemini", ManifestStyle::Generic, true)
    }

    /// Every host noteit knows how to install into.
    pub fn all() -> Vec<NoteitHost> {
        vec![Self::claude(), Self::codex(), Self::gemini()]
    }

    /// The host with the given name, if known.
    pub fn by_name(name: &str) -> Option<NoteitHost> {
        Self::all().into_iter().find(|h| h.name == name)
    }

    /// Number of assets this host writes (excluding the manifest).
    pub fn asset_count(&self) -> usize {
        let mut n = self.assets.all().len();
        if self.include_library_skills {
            n += portable_library_skills(&self.assets).len();
        }
        n
    }

    fn rendered_assets(&self) -> io::Result<Vec<(String, Vec<u8>)>> {
        let mut items = self.assets.all();
        if self.include_library_skills {
            items.extend(portable_library_skills(&self.assets));
        }
        let mut out = Vec::with_capacity(items.len());
        for a in items {
            let bytes = a.render(self.data.clone()).map_err(io::Error::other)?;
            out.push((a.path.clone(), bytes));
        }
        Ok(out)
    }

    fn plugin_json(&self) -> String {
        format!(
            "{{\n  \"name\": \"{}\",\n  \"version\": \"{}\",\n  \"description\": \"{}\"\n}}\n",
            json_escape(&self.data.name),
            json_escape(&self.data.version),
            json_escape(&self.data.description),
        )
    }

    fn manifest_path(&self) -> &'static str {
        match self.manifest {
            ManifestStyle::ClaudePlugin => ".claude-plugin/plugin.json",
            ManifestStyle::Generic => "plugin.json",
        }
    }
}

/// The user home directory, overridable via `NOTEIT_PLUGIN_ROOT` (used for
/// tests and non-standard installs).
fn home_dir() -> io::Result<PathBuf> {
    if let Some(root) = std::env::var_os("NOTEIT_PLUGIN_ROOT") {
        return Ok(PathBuf::from(root));
    }
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "no home directory (USERPROFILE / HOME unset; set NOTEIT_PLUGIN_ROOT)",
            )
        })
}

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

impl TreeWriter for NoteitHost {
    fn walk(&self, f: &mut dyn FnMut(&str, &[u8]) -> io::Result<()>) -> io::Result<()> {
        for (path, data) in self.rendered_assets()? {
            f(&path, &data)?;
        }
        Ok(())
    }

    fn manifest_files(&self) -> io::Result<BTreeMap<String, Vec<u8>>> {
        let mut m = BTreeMap::new();
        m.insert(
            self.manifest_path().to_string(),
            self.plugin_json().into_bytes(),
        );
        Ok(m)
    }
}

impl Host for NoteitHost {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn install_target(&self) -> io::Result<PathBuf> {
        Ok(home_dir()?
            .join(&self.dir_name)
            .join("plugins")
            .join("noteit"))
    }

    fn walk(&self, f: &mut dyn FnMut(&str, &[u8]) -> io::Result<()>) -> io::Result<()> {
        TreeWriter::walk(self, f)
    }

    fn manifest_files(&self) -> io::Result<BTreeMap<String, Vec<u8>>> {
        TreeWriter::manifest_files(self)
    }
}

impl Installer for NoteitHost {
    fn install(&self, target: &std::path::Path) -> io::Result<usize> {
        write_tree_atomic(self, target, SWEEP_DIRS)
    }

    fn uninstall(&self, target: &std::path::Path) -> io::Result<()> {
        match std::fs::remove_dir_all(target) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_hosts_and_lookup() {
        assert_eq!(NoteitHost::all().len(), 3);
        assert_eq!(NoteitHost::claude().name(), "claude");
        assert!(NoteitHost::by_name("gemini").is_some());
        assert!(NoteitHost::by_name("nope").is_none());
    }

    #[test]
    fn claude_uses_native_manifest_and_no_library_skills() {
        let h = NoteitHost::claude();
        assert_eq!(h.manifest_path(), ".claude-plugin/plugin.json");
        // 5 native assets, no synthesized library skills.
        assert_eq!(h.asset_count(), 5);
    }

    #[test]
    fn codex_is_skills_forward_generic_manifest() {
        let h = NoteitHost::codex();
        assert_eq!(h.manifest_path(), "plugin.json");
        // 5 native + 2 library skills.
        assert_eq!(h.asset_count(), 7);
    }

    #[test]
    fn install_writes_tree_and_uninstall_removes_it() {
        let _guard = crate::plugin::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let tmp = std::env::temp_dir().join("noteit-plugin-hosts-test");
        let _ = std::fs::remove_dir_all(&tmp);
        // Isolate the home dir.
        // SAFETY: ENV_LOCK serializes every NOTEIT_PLUGIN_ROOT mutator, so no
        // other thread touches the var while this guard is held.
        unsafe {
            std::env::set_var("NOTEIT_PLUGIN_ROOT", &tmp);
        }

        let h = NoteitHost::claude();
        let target = h.install_target().unwrap();
        let n = h.install(&target).unwrap();
        assert_eq!(n, h.asset_count() + 1, "assets + manifest");
        assert!(target.join(".claude-plugin/plugin.json").exists());
        assert!(target.join("commands/note.md").exists());
        assert!(target.join("skills/noteit/SKILL.md").exists());

        h.uninstall(&target).unwrap();
        assert!(!target.exists());
        // Idempotent.
        h.uninstall(&target).unwrap();

        unsafe {
            std::env::remove_var("NOTEIT_PLUGIN_ROOT");
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn plugin_json_is_wellformed() {
        let h = NoteitHost::claude();
        let j = h.plugin_json();
        assert!(j.contains("\"name\": \"noteit\""));
        assert!(j.contains("\"version\""));
        assert!(j.contains("\"description\""));
    }
}
