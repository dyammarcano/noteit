//! Black-box integration tests for the built `noteit` binary, exercising the
//! `run()` dispatch path (which the in-process `run_core` tests don't cover):
//! the pre-DB plugin branch and a full capture→list→search→delete flow against
//! a real on-disk database. All runs use a temp HOME/USERPROFILE (isolating
//! `noteit.db`) and a temp `NOTEIT_PLUGIN_ROOT` (isolating the install tree).

mod common;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn bin(home: &TempDir, plugin_root: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("noteit").expect("binary must build");
    cmd.env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env("NOTEIT_PLUGIN_ROOT", plugin_root.path());
    cmd
}

#[test]
fn plugin_list_names_every_host() {
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    bin(&home, &root)
        .args(["plugin", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("claude").and(predicates::str::contains("codex")));
}

#[test]
fn plugin_install_status_uninstall_lifecycle() {
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();

    bin(&home, &root)
        .args(["plugin", "install", "--host", "claude"])
        .assert()
        .success()
        .stdout(predicates::str::contains("installed"));

    // The rendered tree exists with the native Claude manifest.
    let plugin_dir = root.path().join(".claude").join("plugins").join("noteit");
    assert!(plugin_dir.join(".claude-plugin/plugin.json").exists());
    assert!(plugin_dir.join("commands/note.md").exists());
    assert!(plugin_dir.join("skills/noteit/SKILL.md").exists());

    bin(&home, &root)
        .args(["plugin", "status", "--host", "claude"])
        .assert()
        .success()
        .stdout(predicates::str::contains("installed"));

    bin(&home, &root)
        .args(["plugin", "uninstall", "--host", "claude"])
        .assert()
        .success()
        .stdout(predicates::str::contains("uninstalled"));

    assert!(!plugin_dir.exists());

    bin(&home, &root)
        .args(["plugin", "status", "--host", "claude"])
        .assert()
        .success()
        .stdout(predicates::str::contains("not installed"));
}

#[test]
fn plugin_install_unknown_host_exits_two() {
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    bin(&home, &root)
        .args(["plugin", "install", "--host", "bogus"])
        .assert()
        .code(2)
        .stderr(predicates::str::contains("unknown host"));
}

#[test]
fn plugin_install_without_host_is_usage_error() {
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    bin(&home, &root)
        .args(["plugin", "install"])
        .assert()
        .code(2);
}

#[test]
fn full_capture_list_search_delete_flow_through_the_binary() {
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let repo = common::repo_with_commits(1);

    // Capture a note (exercises run() DB-open + resolve + run_core::Capture).
    bin(&home, &root)
        .current_dir(repo.path())
        .args(["add", "integration flow note #e2e"])
        .assert()
        .success()
        .stdout(predicates::str::contains("saved"));

    // List shows it.
    bin(&home, &root)
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("integration flow note"));

    // Search finds it.
    bin(&home, &root)
        .current_dir(repo.path())
        .args(["search", "integration"])
        .assert()
        .success()
        .stdout(predicates::str::contains("integration flow note"));

    // Tag filter finds it.
    bin(&home, &root)
        .current_dir(repo.path())
        .args(["list", "--tag", "e2e"])
        .assert()
        .success()
        .stdout(predicates::str::contains("integration flow note"));

    // Delete id 1, then confirm it's gone.
    bin(&home, &root)
        .current_dir(repo.path())
        .args(["delete", "1"])
        .assert()
        .success()
        .stdout(predicates::str::contains("deleted"));

    bin(&home, &root)
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("integration flow note").not());
}
