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
    let idx = msg
        .find(marker)
        .unwrap_or_else(|| panic!("error message did not mention a preserved path: {msg}"));
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

#[test]
fn editor_happy_path_returns_trimmed_body() {
    let _guard = ENV_LOCK.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let script = fake_editor(dir.path(), "a fresh note from the editor", 0);

    // SAFETY: guarded by ENV_LOCK above; no other thread touches EDITOR.
    unsafe {
        std::env::set_var("EDITOR", &script);
    }
    let result = edit_in_editor();
    unsafe {
        std::env::remove_var("EDITOR");
    }

    let body = result.expect("a zero-exit editor must return Ok(body)");
    assert_eq!(body, "a fresh note from the editor");
}

#[test]
fn editor_spawn_failure_is_an_error() {
    let _guard = ENV_LOCK.lock().unwrap();

    // SAFETY: guarded by ENV_LOCK above; no other thread touches EDITOR.
    unsafe {
        std::env::set_var("EDITOR", "noteit-nonexistent-editor-binary-xyz");
    }
    let result = edit_in_editor();
    unsafe {
        std::env::remove_var("EDITOR");
    }

    assert!(
        result.is_err(),
        "spawning a nonexistent editor binary must return an error"
    );
}

#[test]
fn editor_invalid_utf8_surfaces_the_path() {
    let _guard = ENV_LOCK.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();

    // A fake editor that writes invalid UTF-8 bytes and exits 0.
    let script = if cfg!(windows) {
        let script = dir.path().join("fake_editor_bad_utf8.cmd");
        // %~1 gives the path without surrounding quotes.
        let body = "@echo off\r\npowershell -NoProfile -Command \"[IO.File]::WriteAllBytes('%~1', [byte[]](0xFF,0xFE))\"\r\nexit /b 0\r\n".to_string();
        std::fs::write(&script, body).unwrap();
        script
    } else {
        let script = dir.path().join("fake_editor_bad_utf8.sh");
        std::fs::write(&script, "#!/bin/sh\nprintf '\\xff\\xfe' > \"$1\"\nexit 0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&script).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&script, perms).unwrap();
        }
        script
    };

    // SAFETY: guarded by ENV_LOCK above; no other thread touches EDITOR.
    unsafe {
        std::env::set_var("EDITOR", &script);
    }
    let result = edit_in_editor();
    unsafe {
        std::env::remove_var("EDITOR");
    }

    let err = result.expect_err("invalid UTF-8 content must return an error");
    let msg = err.to_string();

    let marker = "could not read note from ";
    let idx = msg
        .find(marker)
        .unwrap_or_else(|| panic!("error message did not mention the temp path: {msg}"));
    let rest = &msg[idx + marker.len()..];
    let end = rest
        .find(" (invalid UTF-8)")
        .unwrap_or_else(|| panic!("error message missing marker: {msg}"));
    let path_str = &rest[..end];
    let preserved_path = std::path::Path::new(path_str);
    assert!(
        preserved_path.exists(),
        "path mentioned in the error does not exist: {path_str}"
    );

    let _ = std::fs::remove_file(preserved_path);
}
