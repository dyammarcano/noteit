//! The host contract a plugin target must satisfy.
//!
//! Ported from an internal Go plugin-host package. [`Host`] is the minimum surface a
//! plugin host implementation exposes so a `noteit plugin install` dispatcher
//! can drive install/uninstall uniformly. [`Installer`], [`Status`], and
//! [`Doctor`] are optional capabilities probed for at the call site.

use std::collections::BTreeMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// The minimum surface a plugin host implementation must expose.
pub trait Host {
    /// Returns the short host identifier (`"claude"`, `"gemini"`, ...).
    fn name(&self) -> String;

    /// Returns the absolute filesystem path where the rendered plugin tree
    /// should be written (under the user's home directory).
    fn install_target(&self) -> io::Result<PathBuf>;

    /// Invokes `f` once per rendered plugin asset (commands, skills, agents —
    /// markdown) with its slash-relative path and bytes. Manifest files come
    /// from [`Host::manifest_files`].
    fn walk(&self, f: &mut dyn FnMut(&str, &[u8]) -> io::Result<()>) -> io::Result<()>;

    /// Returns synthesised host-specific manifest payloads keyed by their
    /// plugin-tree-relative path (e.g. `".claude-plugin/plugin.json"`,
    /// `".mcp.json"`).
    fn manifest_files(&self) -> io::Result<BTreeMap<String, Vec<u8>>>;
}

/// Optional capability for hosts that wire themselves into their CLI's
/// marketplace / settings layer. The dispatcher probes for this so hosts can
/// ship without install plumbing in early stages.
pub trait Installer {
    /// Writes plugin files to `target` and patches host-side state. Returns the
    /// file count written.
    fn install(&self, target: &Path) -> io::Result<usize>;
    /// Removes plugin files at `target` and undoes patches.
    fn uninstall(&self, target: &Path) -> io::Result<()>;
}

/// Optional capability for hosts that report install health checks suitable for
/// a `noteit plugin status` subcommand.
pub trait Status {
    /// Writes a human-readable status report to `w`.
    fn print_status(&self, w: &mut dyn Write) -> io::Result<()>;
}

/// One host-side health-check result.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DoctorCheck {
    /// Check name.
    pub name: String,
    /// `PASS` | `WARN` | `FAIL`.
    pub verdict: String,
    /// Optional detail.
    pub detail: String,
    /// Optional remediation hint.
    pub fix: String,
}

/// Structured output of a host's self-diagnosis.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DoctorReport {
    /// Host identifier.
    pub host: String,
    /// Install target inspected.
    pub target: String,
    /// Individual checks.
    pub checks: Vec<DoctorCheck>,
    /// `OK` | `DEGRADED` | `FAILED`.
    pub verdict: String,
}

/// Optional capability returning host-side health findings.
pub trait Doctor {
    /// Runs the host's self-diagnosis.
    fn doctor(&self) -> DoctorReport;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeHost {
        name: String,
    }

    impl Host for FakeHost {
        fn name(&self) -> String {
            self.name.clone()
        }
        fn install_target(&self) -> io::Result<PathBuf> {
            Ok(PathBuf::from(format!("/tmp/{}", self.name)))
        }
        fn walk(&self, _f: &mut dyn FnMut(&str, &[u8]) -> io::Result<()>) -> io::Result<()> {
            Ok(())
        }
        fn manifest_files(&self) -> io::Result<BTreeMap<String, Vec<u8>>> {
            Ok(BTreeMap::new())
        }
    }

    #[test]
    fn host_trait_object_dispatches() {
        let h: Box<dyn Host> = Box::new(FakeHost {
            name: "claude".into(),
        });
        assert_eq!(h.name(), "claude");
        assert_eq!(h.install_target().unwrap(), PathBuf::from("/tmp/claude"));
        assert!(h.manifest_files().unwrap().is_empty());
    }
}
