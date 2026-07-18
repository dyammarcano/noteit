//! The `noteit plugin` subcommand: install noteit's assets into an AI host.
//!
//! Standalone (no dependency on [`crate::cli`]) so it can be parsed there and
//! dispatched here without a cycle. Filesystem-only — never touches the notes
//! database.

use std::io::{self, Write};

use super::host::{Host, Installer};
use super::hosts::NoteitHost;

/// Which host(s) a plugin operation targets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostSel {
    /// A single named host.
    One(String),
    /// Every known host.
    All,
}

/// A parsed `noteit plugin ...` operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginCmd {
    /// List known hosts and their install targets.
    List,
    /// Install noteit's assets into the selected host(s).
    Install(HostSel),
    /// Report install status for the selected host(s), or all.
    Status(Option<HostSel>),
    /// Remove noteit's assets from the selected host(s).
    Uninstall(HostSel),
}

fn resolve(sel: &HostSel) -> Vec<NoteitHost> {
    match sel {
        HostSel::All => NoteitHost::all(),
        HostSel::One(name) => match NoteitHost::by_name(name) {
            Some(h) => vec![h],
            None => {
                // Diagnostics go to stderr; stdout is reserved for operation
                // output (noteit's stream convention).
                let known: Vec<String> = NoteitHost::all().iter().map(|h| h.name()).collect();
                eprintln!("unknown host: {name} (known: {})", known.join(", "));
                vec![]
            }
        },
    }
}

/// Runs a parsed [`PluginCmd`]. Returns the process exit code.
pub fn run(cmd: &PluginCmd, out: &mut dyn Write) -> io::Result<i32> {
    match cmd {
        PluginCmd::List => {
            writeln!(out, "known hosts:")?;
            for h in NoteitHost::all() {
                let target = h.install_target()?;
                writeln!(
                    out,
                    "  {:<7} {} assets -> {}",
                    h.name(),
                    h.asset_count(),
                    target.display()
                )?;
            }
            Ok(0)
        }
        PluginCmd::Install(sel) => {
            let hosts = resolve(sel);
            if hosts.is_empty() {
                return Ok(2);
            }
            for h in &hosts {
                let target = h.install_target()?;
                let n = h.install(&target)?;
                writeln!(
                    out,
                    "installed {n} files for {} -> {}",
                    h.name(),
                    target.display()
                )?;
            }
            Ok(0)
        }
        PluginCmd::Uninstall(sel) => {
            let hosts = resolve(sel);
            if hosts.is_empty() {
                return Ok(2);
            }
            for h in &hosts {
                let target = h.install_target()?;
                h.uninstall(&target)?;
                writeln!(out, "uninstalled {} -> {}", h.name(), target.display())?;
            }
            Ok(0)
        }
        PluginCmd::Status(sel) => {
            let hosts = match sel {
                Some(s) => resolve(s),
                None => NoteitHost::all(),
            };
            if hosts.is_empty() {
                return Ok(2);
            }
            for h in &hosts {
                let target = h.install_target()?;
                let (state, detail) = if target.exists() {
                    let count = installed_file_count(&target);
                    ("installed", format!("{count} files"))
                } else {
                    ("not installed", String::new())
                };
                writeln!(
                    out,
                    "  {:<7} {state} {detail} ({})",
                    h.name(),
                    target.display()
                )?;
            }
            Ok(0)
        }
    }
}

fn installed_file_count(dir: &std::path::Path) -> usize {
    let mut n = 0;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                n += 1;
            }
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_isolated_home<T>(tag: &str, f: impl FnOnce() -> T) -> T {
        let _guard = crate::plugin::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let tmp = std::env::temp_dir().join(format!("noteit-plugin-cmd-{tag}"));
        let _ = std::fs::remove_dir_all(&tmp);
        // SAFETY: ENV_LOCK serializes every NOTEIT_PLUGIN_ROOT mutator, so no
        // other thread reads or writes the var while this guard is held.
        unsafe {
            std::env::set_var("NOTEIT_PLUGIN_ROOT", &tmp);
        }
        let r = f();
        unsafe {
            std::env::remove_var("NOTEIT_PLUGIN_ROOT");
        }
        let _ = std::fs::remove_dir_all(&tmp);
        r
    }

    #[test]
    fn list_names_every_host() {
        let mut out = Vec::new();
        with_isolated_home("list", || run(&PluginCmd::List, &mut out).unwrap());
        let s = String::from_utf8(out).unwrap();
        assert!(s.contains("claude"));
        assert!(s.contains("codex"));
        assert!(s.contains("gemini"));
    }

    #[test]
    fn install_then_status_then_uninstall() {
        with_isolated_home("lifecycle", || {
            let mut out = Vec::new();
            let code = run(&PluginCmd::Install(HostSel::One("claude".into())), &mut out).unwrap();
            assert_eq!(code, 0);
            assert!(String::from_utf8(out).unwrap().contains("installed"));

            let mut out = Vec::new();
            run(
                &PluginCmd::Status(Some(HostSel::One("claude".into()))),
                &mut out,
            )
            .unwrap();
            assert!(String::from_utf8(out).unwrap().contains("installed"));

            let mut out = Vec::new();
            run(
                &PluginCmd::Uninstall(HostSel::One("claude".into())),
                &mut out,
            )
            .unwrap();
            assert!(String::from_utf8(out).unwrap().contains("uninstalled"));

            let mut out = Vec::new();
            run(
                &PluginCmd::Status(Some(HostSel::One("claude".into()))),
                &mut out,
            )
            .unwrap();
            assert!(String::from_utf8(out).unwrap().contains("not installed"));
        });
    }

    #[test]
    fn install_all_hosts() {
        with_isolated_home("all", || {
            let mut out = Vec::new();
            let code = run(&PluginCmd::Install(HostSel::All), &mut out).unwrap();
            assert_eq!(code, 0);
            let s = String::from_utf8(out).unwrap();
            assert_eq!(s.matches("installed").count(), 3);
        });
    }

    #[test]
    fn unknown_host_reports_and_fails() {
        // The "unknown host" diagnostic goes to stderr (not captured here); an
        // unknown host must yield exit code 2 and write nothing to stdout.
        let mut out = Vec::new();
        let code = with_isolated_home("unknown", || {
            run(&PluginCmd::Install(HostSel::One("bogus".into())), &mut out).unwrap()
        });
        assert_eq!(code, 2);
        assert!(out.is_empty(), "no stdout on unknown host");
    }
}
