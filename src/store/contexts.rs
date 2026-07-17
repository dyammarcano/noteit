use rusqlite::{params, Row};

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
                 VALUES (?1, ?2, ?3, 0, ?4, 0, ?5)",
                params![kind.as_str(), key, display_name, root_path, now()],
            )
            .map_err(StoreError::Sqlite)?;
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

    /// Path contexts at or under `root`. Used by adoption.
    pub fn path_contexts_under(&self, root: &str) -> Result<Vec<Context>, StoreError> {
        let sql = format!(
            "SELECT {SELECT_COLS} FROM contexts
             WHERE kind = 'path' AND (key = ?1 OR key LIKE ?2 ESCAPE '\\')"
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
}
