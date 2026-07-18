//! Black-box tests for env-driven config (`NOTEIT_DB`, `NOTEIT_QUIET`) and the
//! `export` verb, run against the built binary.

mod common;

use assert_cmd::Command;
use predicates::prelude::*;
use std::path::Path;

fn bin(db: &Path) -> Command {
    let mut c = Command::cargo_bin("noteit").expect("binary must build");
    // NOTEIT_DB wins; HOME/USERPROFILE are a fallback safety net so the test
    // never touches the real ~/noteit.db even if the override logic changed.
    let home = db.parent().unwrap();
    c.env("NOTEIT_DB", db)
        .env("HOME", home)
        .env("USERPROFILE", home);
    c
}

#[test]
fn noteit_db_env_directs_the_database() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("custom.db");
    let repo = common::repo_with_commits(1);

    bin(&db)
        .current_dir(repo.path())
        .args(["add", "note in custom db"])
        .assert()
        .success();
    assert!(db.exists(), "NOTEIT_DB path should have been created");

    // A second process pointed at the same DB sees the note.
    bin(&db)
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("note in custom db"));
}

#[test]
fn export_emits_json_with_all_notes() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("e.db");
    let repo = common::repo_with_commits(1);

    for body in ["first note #a", "second note"] {
        bin(&db)
            .current_dir(repo.path())
            .args(["add", body])
            .assert()
            .success();
    }

    bin(&db)
        .current_dir(repo.path())
        .arg("export")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("\"noteit_export\": \"v1\"")
                .and(predicate::str::contains("first note #a"))
                .and(predicate::str::contains("second note"))
                .and(predicate::str::contains("\"status\": \"open\"")),
        );
}

#[test]
fn export_on_empty_db_is_valid_empty_json() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("empty.db");
    let repo = common::repo_with_commits(1);
    bin(&db)
        .current_dir(repo.path())
        .arg("export")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"notes\": []"));
}

#[test]
fn quiet_suppresses_adoption_notice() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("q.db");
    let repo = common::empty_repo();

    // Capture while path-bound (repo has no commits yet).
    bin(&db)
        .current_dir(repo.path())
        .args(["add", "pre-repo note"])
        .assert()
        .success();

    // Give it a commit so the next run adopts the path context's note.
    common::commit_file(repo.path(), "f0.txt", "v0");

    // With NOTEIT_QUIET set, adoption still happens but prints no notice.
    bin(&db)
        .current_dir(repo.path())
        .env("NOTEIT_QUIET", "1")
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("pre-repo note"))
        .stderr(predicate::str::is_empty());
}
