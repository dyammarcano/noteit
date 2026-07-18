//! Context storage: the repo/path contexts notes bind to — upsert, lookup,
//! rename, and the adoption bookkeeping (folding path contexts into a repo).

use rusqlite::{Row, params};

use super::{Store, StoreError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Repo,
    Path,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Repo => "repo",
            Kind::Path => "path",
        }
    }

    pub fn parse(s: &str) -> Kind {
        match s {
            "repo" => Kind::Repo,
            _ => Kind::Path,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Context {
    pub id: i64,
    pub kind: Kind,
    pub key: String,
    pub display_name: String,
    pub name_overridden: bool,
    pub root_path: String,
    pub shallow_warned: bool,
}

/// Map 7 consecutive columns starting at `offset` into a Context.
///
/// THE single context row-mapper. `notes.rs` joins contexts alongside notes
/// and calls this with its own offset -- do not write a second copy there.
pub(crate) fn row_to_context(row: &Row<'_>, offset: usize) -> rusqlite::Result<Context> {
    Ok(Context {
        id: row.get(offset)?,
        kind: Kind::parse(&row.get::<_, String>(offset + 1)?),
        key: row.get(offset + 2)?,
        display_name: row.get(offset + 3)?,
        name_overridden: row.get::<_, i64>(offset + 4)? != 0,
        root_path: row.get(offset + 5)?,
        shallow_warned: row.get::<_, i64>(offset + 6)? != 0,
    })
}

/// Context columns, unaliased -- for queries selecting from `contexts` alone.
pub(crate) const SELECT_COLS: &str =
    "id, kind, key, display_name, name_overridden, root_path, shallow_warned";

/// Context columns aliased to `c` -- for joins in `notes.rs`. Same order as
/// SELECT_COLS, so `row_to_context` maps either one.
pub(crate) const CTX_COLS: &str =
    "c.id, c.kind, c.key, c.display_name, c.name_overridden, c.root_path, c.shallow_warned";

use super::now;

impl Store {
    pub fn find_context(&self, kind: Kind, key: &str) -> Result<Option<Context>, StoreError> {
        let sql = format!("SELECT {SELECT_COLS} FROM contexts WHERE kind = ?1 AND key = ?2");
        self.conn()
            .query_row(&sql, params![kind.as_str(), key], |r| row_to_context(r, 0))
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(StoreError::Sqlite(other)),
            })
    }

    pub fn upsert_context(
        &self,
        kind: Kind,
        key: &str,
        display_name: &str,
        root_path: &str,
    ) -> Result<Context, StoreError> {
        if let Some(existing) = self.find_context(kind, key)? {
            return Ok(existing);
        }
        self.conn()
            .execute(
                "INSERT INTO contexts (kind, key, display_name, name_overridden, root_path, shallow_warned, created_at)
                 VALUES (?1, ?2, ?3, 0, ?4, 0, ?5)
                 ON CONFLICT(kind, key) DO NOTHING",
                params![kind.as_str(), key, display_name, root_path, now()],
            )
            .map_err(StoreError::Sqlite)?;
        // A concurrent writer may have raced us between the find_context
        // above and this INSERT and won -- ON CONFLICT DO NOTHING makes that
        // harmless, and this read-back returns whichever row exists now
        // (ours, or the racing writer's) instead of propagating
        // SQLITE_CONSTRAINT_UNIQUE and losing the caller's note.
        self.find_context(kind, key)?
            .ok_or_else(|| StoreError::Sqlite(rusqlite::Error::QueryReturnedNoRows))
    }

    pub fn rename_context(&self, id: i64, name: &str) -> Result<(), StoreError> {
        self.conn()
            .execute(
                "UPDATE contexts SET display_name = ?1, name_overridden = 1 WHERE id = ?2",
                params![name, id],
            )
            .map_err(StoreError::Sqlite)?;
        Ok(())
    }

    /// Mark a context's shallow warning as delivered. Returns true if this
    /// call was the one that flipped it (i.e. the caller should warn now).
    pub fn claim_shallow_warning(&self, id: i64) -> Result<bool, StoreError> {
        let n = self
            .conn()
            .execute(
                "UPDATE contexts SET shallow_warned = 1 WHERE id = ?1 AND shallow_warned = 0",
                params![id],
            )
            .map_err(StoreError::Sqlite)?;
        Ok(n > 0)
    }

    /// Path contexts at or under `root`. Used by adoption.
    pub fn path_contexts_under(&self, root: &str) -> Result<Vec<Context>, StoreError> {
        let sql = format!(
            "SELECT {SELECT_COLS} FROM contexts
             WHERE kind = 'path' AND no_adopt = 0 AND (key = ?1 OR key LIKE ?2 ESCAPE '\\')"
        );
        let root_with_sep = format!("{root}{}", std::path::MAIN_SEPARATOR);
        let prefix = format!(
            "{}%",
            root_with_sep
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_")
        );
        let conn = self.conn();
        let mut stmt = conn.prepare(&sql).map_err(StoreError::Sqlite)?;
        let rows = stmt
            .query_map(params![root, prefix], |r| row_to_context(r, 0))
            .map_err(StoreError::Sqlite)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(StoreError::Sqlite)?);
        }
        Ok(out)
    }

    /// Fold path contexts into a repo context in ONE transaction.
    ///
    /// Notes never change tables -- only `context_id` and `subpath` -- so a
    /// note's identity survives adoption. Returns notes moved.
    pub fn adopt(
        &mut self,
        from: &[Context],
        to_context_id: i64,
        to_root: &str,
    ) -> Result<usize, StoreError> {
        if from.is_empty() {
            return Ok(0);
        }
        let ts = now();
        let tx = self.conn.transaction().map_err(StoreError::Sqlite)?;
        let mut moved = 0usize;

        for ctx in from {
            // Subpath of the old path context relative to the new repo root.
            let subpath = subpath_of(to_root, &ctx.root_path);

            let ids: Vec<i64> = {
                let mut stmt = tx
                    .prepare("SELECT id FROM notes WHERE context_id = ?1")
                    .map_err(StoreError::Sqlite)?;
                let rows = stmt
                    .query_map(params![ctx.id], |r| r.get::<_, i64>(0))
                    .map_err(StoreError::Sqlite)?;
                let mut v = Vec::new();
                for r in rows {
                    v.push(r.map_err(StoreError::Sqlite)?);
                }
                v
            };

            if ids.is_empty() {
                // Nothing to move, but the stale path context still goes --
                // and it still gets an audit row, so an empty fold is never
                // silently destructive (undo needs its identity too).
                tx.execute(
                    "INSERT INTO adoptions
                     (from_context_id, to_context_id, note_ids, adopted_at,
                      from_key, from_root_path, from_display_name, from_name_overridden)
                     VALUES (?1, ?2, '', ?3, ?4, ?5, ?6, ?7)",
                    params![
                        ctx.id,
                        to_context_id,
                        ts,
                        ctx.key,
                        ctx.root_path,
                        ctx.display_name,
                        ctx.name_overridden as i64,
                    ],
                )
                .map_err(StoreError::Sqlite)?;
                tx.execute("DELETE FROM contexts WHERE id = ?1", params![ctx.id])
                    .map_err(StoreError::Sqlite)?;
                continue;
            }

            // Invariant this UPDATE relies on: every note in a path context
            // shares subpath "." (resolve() hardcodes it), so blanket-setting
            // subpath here for all of the context's notes is safe.
            let n = tx
                .execute(
                    "UPDATE notes SET context_id = ?1, subpath = ?2 WHERE context_id = ?3",
                    params![to_context_id, subpath, ctx.id],
                )
                .map_err(StoreError::Sqlite)?;
            moved += n;

            let id_list = ids
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(",");
            tx.execute(
                "INSERT INTO adoptions
                 (from_context_id, to_context_id, note_ids, adopted_at,
                  from_key, from_root_path, from_display_name, from_name_overridden)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    ctx.id,
                    to_context_id,
                    id_list,
                    ts,
                    ctx.key,
                    ctx.root_path,
                    ctx.display_name,
                    ctx.name_overridden as i64,
                ],
            )
            .map_err(StoreError::Sqlite)?;

            tx.execute("DELETE FROM contexts WHERE id = ?1", params![ctx.id])
                .map_err(StoreError::Sqlite)?;
        }

        tx.commit().map_err(StoreError::Sqlite)?;
        Ok(moved)
    }

    /// Undo the most recent adoption batch into `to_context_id`: recreate
    /// (and pin) each folded path context, move its notes back, and delete
    /// the audit rows. Pinning (`no_adopt = 1`) is what stops the very next
    /// automatic adoption from silently re-folding it -- see
    /// `path_contexts_under`'s `no_adopt = 0` filter.
    pub fn undo_last_adoption(
        &mut self,
        to_context_id: i64,
    ) -> Result<Option<UndoReport>, StoreError> {
        let tx = self.conn.transaction().map_err(StoreError::Sqlite)?;

        struct Row {
            id: i64,
            note_ids: String,
            from_key: String,
            from_root_path: String,
            from_display_name: String,
            from_name_overridden: i64,
        }

        let rows: Vec<Row> = {
            let mut stmt = tx
                .prepare(
                    "SELECT id, note_ids, from_key, from_root_path, from_display_name, from_name_overridden
                     FROM adoptions
                     WHERE to_context_id = ?1
                       AND adopted_at = (SELECT MAX(adopted_at) FROM adoptions WHERE to_context_id = ?1)",
                )
                .map_err(StoreError::Sqlite)?;
            let mapped = stmt
                .query_map(params![to_context_id], |r| {
                    Ok(Row {
                        id: r.get(0)?,
                        note_ids: r.get(1)?,
                        from_key: r.get(2)?,
                        from_root_path: r.get(3)?,
                        from_display_name: r.get(4)?,
                        from_name_overridden: r.get(5)?,
                    })
                })
                .map_err(StoreError::Sqlite)?;
            let mut v = Vec::new();
            for r in mapped {
                v.push(r.map_err(StoreError::Sqlite)?);
            }
            v
        };

        if rows.is_empty() {
            tx.commit().map_err(StoreError::Sqlite)?;
            return Ok(None);
        }

        let ts = now();
        let mut notes_restored = 0usize;
        let mut paths_restored = 0usize;

        for row in &rows {
            tx.execute(
                "INSERT INTO contexts (kind, key, display_name, name_overridden, root_path, shallow_warned, no_adopt, created_at)
                 VALUES ('path', ?1, ?2, ?3, ?4, 0, 1, ?5)
                 ON CONFLICT(kind, key) DO UPDATE SET no_adopt = 1",
                params![row.from_key, row.from_display_name, row.from_name_overridden, row.from_root_path, ts],
            )
            .map_err(StoreError::Sqlite)?;

            let restored_ctx_id: i64 = tx
                .query_row(
                    "SELECT id FROM contexts WHERE kind = 'path' AND key = ?1",
                    params![row.from_key],
                    |r| r.get(0),
                )
                .map_err(StoreError::Sqlite)?;

            if !row.note_ids.is_empty() {
                for id_str in row.note_ids.split(',') {
                    let note_id: i64 = id_str.parse().map_err(|_| {
                        StoreError::Sqlite(rusqlite::Error::InvalidColumnType(
                            0,
                            "note_ids".to_string(),
                            rusqlite::types::Type::Text,
                        ))
                    })?;
                    let n = tx
                        .execute(
                            "UPDATE notes SET context_id = ?1, subpath = '.' WHERE id = ?2",
                            params![restored_ctx_id, note_id],
                        )
                        .map_err(StoreError::Sqlite)?;
                    notes_restored += n;
                }
            }

            tx.execute("DELETE FROM adoptions WHERE id = ?1", params![row.id])
                .map_err(StoreError::Sqlite)?;
            paths_restored += 1;
        }

        tx.commit().map_err(StoreError::Sqlite)?;
        Ok(Some(UndoReport {
            notes_restored,
            paths_restored,
        }))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UndoReport {
    pub notes_restored: usize,
    pub paths_restored: usize,
}

/// Path of `child` relative to `root`, in noteit's subpath form.
pub fn subpath_of(root: &str, child: &str) -> String {
    let root_p = std::path::Path::new(root);
    let child_p = std::path::Path::new(child);
    match child_p.strip_prefix(root_p) {
        Ok(p) if p.as_os_str().is_empty() => ".".to_string(),
        Ok(p) => p.to_string_lossy().replace('\\', "/"),
        Err(_) => ".".to_string(),
    }
}
