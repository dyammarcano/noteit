use noteit::cli::edit_in_editor;
use std::io::Write;

/// Writes a fake "editor" script that appends `text` to the file path it
/// receives as its first argument, then exits with `exit_code`.
fn fake_editor(dir: &std::path::Path, text: &str, exit_code: i32) -> std::path::PathBuf {
    if cfg!(windows) {
        let script = dir.join("fake_editor.cmd");
        let mut f = std::fs::File::create(&script).unwrap();
        // %~1 strips quotes so paths with spaces still work.
        writeln!(f, "@echo off").unwrap();
        writeln!(f, ">> \"%~1\" echo {text}").unwrap();
        writeln!(f, "exit /b {exit_code}").unwrap();
        script
    } else {
        let script = dir.join("fake_editor.sh");
        let mut f = std::fs::File::create(&script).unwrap();
        writeln!(f, "#!/bin/sh").unwrap();
        writeln!(f, "printf '%s' \"{text}\" >> \"$1\"").unwrap();
        writeln!(f, "exit {exit_code}").unwrap();
        drop(f);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&script).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&script, perms).unwrap();
        }
        script
    }
}

// These tests set/remove process-wide env vars ($EDITOR) so they must not
// run concurrently with each other or with anything else touching $EDITOR.
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn editor_nonzero_exit_preserves_typed_text() {
    let _guard = ENV_LOCK.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let script = fake_editor(dir.path(), "some typed note text", 1);

    // SAFETY: guarded by ENV_LOCK above; no other thread touches EDITOR.
    unsafe {
        std::env::set_var("EDITOR", &script);
    }
    let result = edit_in_editor();
    unsafe {
        std::env::remove_var("EDITOR");
    }

    let err = result.expect_err("non-zero editor exit must return an error");
    let msg = err.to_string();

    // Extract the preserved path from the error message and verify the
    // typed text is still there -- the core "never lose a note" guarantee.
    let marker = "preserved at ";
    let idx = msg.find(marker).unwrap_or_else(|| panic!("error message did not mention a preserved path: {msg}"));
    let path_str = msg[idx + marker.len()..].trim();
    let preserved_path = std::path::Path::new(path_str);
    assert!(
        preserved_path.exists(),
        "preserved path {path_str} does not exist; text was lost"
    );
    let contents = std::fs::read_to_string(preserved_path).unwrap();
    assert!(
        contents.contains("some typed note text"),
        "preserved file did not contain the typed text: {contents:?}"
    );

    let _ = std::fs::remove_file(preserved_path);
}
