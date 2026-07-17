use assert_cmd::Command;

/// A single black-box smoke test for the built `noteit` binary: `--version`
/// exits 0 and prints the crate name. Runs against a temp HOME/USERPROFILE
/// so it never touches the real `noteit.db`, even though `--version` is
/// handled before the DB is opened (see `cli::run`'s doc comment).
#[test]
fn version_flag_exits_zero_and_prints_the_binary_name() {
    let home = tempfile::tempdir().expect("tempdir");

    let mut cmd = Command::cargo_bin("noteit").expect("binary must build");
    cmd.arg("--version")
        .env("HOME", home.path())
        .env("USERPROFILE", home.path());

    cmd.assert()
        .success()
        .stdout(predicates::str::contains("noteit"));
}
