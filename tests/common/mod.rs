#![allow(dead_code)]
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

/// Run a git command in `dir`, panicking with stderr on failure.
pub fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git must be on PATH");
    assert!(
        out.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// A temp dir that is NOT a git repo.
pub fn plain_dir() -> TempDir {
    tempfile::tempdir().expect("tempdir")
}

/// A temp git repo with zero commits.
pub fn empty_repo() -> TempDir {
    let td = tempfile::tempdir().expect("tempdir");
    git(td.path(), &["init", "-q"]);
    git(td.path(), &["symbolic-ref", "HEAD", "refs/heads/master"]);
    git(td.path(), &["config", "user.name", "Test"]);
    git(td.path(), &["config", "user.email", "test@example.com"]);
    td
}

/// A temp git repo with `n` sequential commits on the default branch.
pub fn repo_with_commits(n: usize) -> TempDir {
    let td = empty_repo();
    for i in 0..n {
        commit_file(td.path(), &format!("f{i}.txt"), &format!("v{i}"));
    }
    td
}

/// Write a file and commit it.
pub fn commit_file(dir: &Path, name: &str, body: &str) -> String {
    std::fs::write(dir.join(name), body).expect("write");
    git(dir, &["add", name]);
    git(dir, &["commit", "-q", "-m", &format!("add {name}")]);
    git(dir, &["rev-parse", "HEAD"])
}

/// The root commit sha git itself reports (the parity oracle).
pub fn git_root_sha(dir: &Path) -> String {
    let out = git(dir, &["rev-list", "--max-parents=0", "HEAD"]);
    let mut roots: Vec<&str> = out.lines().map(str::trim).collect();
    roots.sort_unstable();
    roots[0].to_string()
}

pub fn path_of(td: &TempDir) -> PathBuf {
    td.path().to_path_buf()
}
