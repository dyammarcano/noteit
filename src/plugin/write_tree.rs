//! Atomic asset-tree writer with stale-sweep.
//!
//! Ported from an internal Go plugin-host package. [`write_tree_atomic`] is the
//! shared writer used by hosts that need no host-specific install ritual: every
//! file from [`TreeWriter::walk`] + [`TreeWriter::manifest_files`] is written via
//! tmp+rename, then any pre-existing file under `sweep_dirs` not produced this
//! run is removed (so an install over an older tree drops stale assets).

use std::collections::HashSet;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{MAIN_SEPARATOR_STR, Path, PathBuf};

/// The host surface [`write_tree_atomic`] needs: the rendered asset tree
/// (`walk`) plus the manifest files (`manifest_files`).
pub trait TreeWriter {
    /// Invokes `f` once per rendered asset with its slash-relative path and
    /// bytes. A non-`Ok` `f` return aborts the walk.
    fn walk(&self, f: &mut dyn FnMut(&str, &[u8]) -> io::Result<()>) -> io::Result<()>;
    /// Returns the plugin manifest files keyed by slash-relative path.
    fn manifest_files(&self) -> io::Result<std::collections::BTreeMap<String, Vec<u8>>>;
}

/// Writes every asset + manifest file from `h` into `target` using tmp+rename,
/// then sweeps stale files under `sweep_dirs` (relative to `target`). Returns
/// the number of files written.
pub fn write_tree_atomic(
    h: &dyn TreeWriter,
    target: &Path,
    sweep_dirs: &[&str],
) -> io::Result<usize> {
    fs::create_dir_all(target)?;

    let mut wanted: HashSet<PathBuf> = HashSet::new();
    let mut count: usize = 0;

    // Scoped so the closure's mutable borrows of `wanted`/`count` end before
    // the sweep phase reads them.
    {
        let mut write_one = |rel: &str, data: &[u8]| -> io::Result<()> {
            let dst = target.join(rel.replace('/', MAIN_SEPARATOR_STR));
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent)?;
            }
            atomic_write_file(&dst, data)?;
            wanted.insert(clean_path(&dst));
            count += 1;
            Ok(())
        };

        h.walk(&mut write_one)?;

        let manifest = h.manifest_files()?;
        for (rel, data) in &manifest {
            write_one(rel, data)?;
        }
    }

    for sub in sweep_dirs {
        let sub_path = target.join(sub);
        let mut files = Vec::new();
        collect_files(&sub_path, &mut files);
        for p in files {
            if !wanted.contains(&clean_path(&p)) && fs::remove_file(&p).is_ok() {
                eprintln!("[install] swept stale: {}", p.display());
            }
        }
    }

    Ok(count)
}

/// Writes `data` to `path` atomically: write `<path>.tmp` then rename over
/// `path`.
fn atomic_write_file(path: &Path, data: &[u8]) -> io::Result<()> {
    let mut tmp: OsString = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    fs::write(&tmp, data)?;
    fs::rename(&tmp, path)
}

/// Canonicalizes `p` for set membership, falling back to the raw path if the
/// file cannot be resolved.
fn clean_path(p: &Path) -> PathBuf {
    fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

/// Recursively collects every file (not directory) under `dir`.
fn collect_files(dir: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            collect_files(&p, into);
        } else {
            into.push(p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    struct FixtureWriter {
        files: Vec<(String, Vec<u8>)>,
        manifest: BTreeMap<String, Vec<u8>>,
    }

    impl TreeWriter for FixtureWriter {
        fn walk(&self, f: &mut dyn FnMut(&str, &[u8]) -> io::Result<()>) -> io::Result<()> {
            for (path, data) in &self.files {
                f(path, data)?;
            }
            Ok(())
        }
        fn manifest_files(&self) -> io::Result<BTreeMap<String, Vec<u8>>> {
            Ok(self.manifest.clone())
        }
    }

    fn tmp_target(tag: &str) -> PathBuf {
        let mut base = std::env::temp_dir();
        // Unique-enough per test name; tests here use distinct tags.
        base.push(format!("noteit-plugin-wt-{tag}"));
        let _ = fs::remove_dir_all(&base);
        base
    }

    #[test]
    fn writes_assets_and_manifest_and_counts() {
        let target = tmp_target("write");
        let mut manifest = BTreeMap::new();
        manifest.insert(".mcp.json".to_string(), b"{}".to_vec());
        let w = FixtureWriter {
            files: vec![
                ("commands/a.md".into(), b"a".to_vec()),
                ("skills/s/SKILL.md".into(), b"s".to_vec()),
            ],
            manifest,
        };

        let n = write_tree_atomic(&w, &target, &["commands", "skills"]).unwrap();
        assert_eq!(n, 3, "2 assets + 1 manifest");
        assert_eq!(fs::read(target.join("commands/a.md")).unwrap(), b"a");
        assert_eq!(fs::read(target.join("skills/s/SKILL.md")).unwrap(), b"s");
        assert_eq!(fs::read(target.join(".mcp.json")).unwrap(), b"{}");

        let _ = fs::remove_dir_all(&target);
    }

    #[test]
    fn sweeps_stale_files_under_sweep_dirs() {
        let target = tmp_target("sweep");
        let w1 = FixtureWriter {
            files: vec![
                ("commands/a.md".into(), b"a".to_vec()),
                ("commands/b.md".into(), b"b".to_vec()),
            ],
            manifest: BTreeMap::new(),
        };
        write_tree_atomic(&w1, &target, &["commands"]).unwrap();
        assert!(target.join("commands/b.md").exists());

        // Second install drops b.md — it must be swept.
        let w2 = FixtureWriter {
            files: vec![("commands/a.md".into(), b"a2".to_vec())],
            manifest: BTreeMap::new(),
        };
        write_tree_atomic(&w2, &target, &["commands"]).unwrap();
        assert!(target.join("commands/a.md").exists(), "kept");
        assert!(!target.join("commands/b.md").exists(), "swept stale");
        assert_eq!(
            fs::read(target.join("commands/a.md")).unwrap(),
            b"a2",
            "overwritten"
        );

        let _ = fs::remove_dir_all(&target);
    }
}
