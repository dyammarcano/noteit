use rusqlite::Connection;

use super::StoreError;

/// Each entry is one migration, applied in order. `user_version` records
/// how many have run. Append only -- never edit a shipped migration.
const MIGRATIONS: &[&str] = &[
    // v1: initial schema
    r#"
    CREATE TABLE contexts (
      id              INTEGER PRIMARY KEY,
      kind            TEXT    NOT NULL CHECK(kind IN ('repo','path')),
      key             TEXT    NOT NULL,
      display_name    TEXT    NOT NULL,
      name_overridden INTEGER NOT NULL DEFAULT 0,
      root_path       TEXT    NOT NULL,
      shallow_warned  INTEGER NOT NULL DEFAULT 0,
      created_at      INTEGER NOT NULL,
      UNIQUE(kind, key)
    );

    CREATE TABLE notes (
      id         INTEGER PRIMARY KEY,
      context_id INTEGER NOT NULL REFERENCES contexts(id),
      subpath    TEXT    NOT NULL,
      body       TEXT    NOT NULL,
      status     TEXT    NOT NULL DEFAULT 'open' CHECK(status IN ('open','done')),
      created_at INTEGER NOT NULL,
      updated_at INTEGER NOT NULL
    );

    CREATE INDEX idx_notes_context_subpath ON notes(context_id, subpath);

    CREATE TABLE tags (
      id   INTEGER PRIMARY KEY,
      name TEXT NOT NULL UNIQUE
    );

    CREATE TABLE note_tags (
      note_id INTEGER NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
      tag_id  INTEGER NOT NULL REFERENCES tags(id),
      PRIMARY KEY(note_id, tag_id)
    );

    CREATE TABLE adoptions (
      id                    INTEGER PRIMARY KEY,
      from_context_id       INTEGER NOT NULL,
      to_context_id         INTEGER NOT NULL,
      note_ids              TEXT    NOT NULL,
      adopted_at            INTEGER NOT NULL,
      from_key              TEXT    NOT NULL,
      from_root_path        TEXT    NOT NULL,
      from_display_name     TEXT    NOT NULL,
      from_name_overridden  INTEGER NOT NULL DEFAULT 0
    );

    -- External-content FTS5: the index mirrors notes.body, kept current
    -- by the triggers below. `content_rowid` ties it to notes.id.
    CREATE VIRTUAL TABLE notes_fts USING fts5(
      body,
      content='notes',
      content_rowid='id'
    );

    CREATE TRIGGER notes_ai AFTER INSERT ON notes BEGIN
      INSERT INTO notes_fts(rowid, body) VALUES (new.id, new.body);
    END;

    CREATE TRIGGER notes_ad AFTER DELETE ON notes BEGIN
      INSERT INTO notes_fts(notes_fts, rowid, body) VALUES ('delete', old.id, old.body);
    END;

    CREATE TRIGGER notes_au AFTER UPDATE ON notes BEGIN
      INSERT INTO notes_fts(notes_fts, rowid, body) VALUES ('delete', old.id, old.body);
      INSERT INTO notes_fts(rowid, body) VALUES (new.id, new.body);
    END;
    "#,
    // v2: pin flag so undo-recreated path contexts are never re-adopted
    r#"
    ALTER TABLE contexts ADD COLUMN no_adopt INTEGER NOT NULL DEFAULT 0;
    "#,
];

pub fn migrate(conn: &mut Connection) -> Result<(), StoreError> {
    let current: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(StoreError::Sqlite)?;

    for (i, sql) in MIGRATIONS.iter().enumerate() {
        let version = (i + 1) as i64;
        if version <= current {
            continue;
        }
        let tx = conn.transaction().map_err(StoreError::Sqlite)?;
        tx.execute_batch(sql).map_err(|e| StoreError::Migration {
            at: version,
            source: e,
        })?;
        tx.pragma_update(None, "user_version", version)
            .map_err(|e| StoreError::Migration {
                at: version,
                source: e,
            })?;
        tx.commit().map_err(StoreError::Sqlite)?;
    }
    Ok(())
}
