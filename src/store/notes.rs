use rusqlite::{params, Row};

// Shared helpers -- do NOT re-declare these here. `row_to_context` and
// `CTX_COLS` are defined once in contexts.rs; `now()` once in mod.rs.
use super::contexts::{row_to_context, Context, CTX_COLS};
use super::{now, Store, StoreError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Open,
    Done,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Open => "open",
            Status::Done => "done",
        }
    }

    pub fn parse(s: &str) -> Status {
        match s {
            "done" => Status::Done,
            _ => Status::Open,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Note {
    pub id: i64,
    pub context_id: i64,
    pub subpath: String,
    pub body: String,
    pub status: Status,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Extract `#tags` from a note body: lowercased, deduped, order preserved.
///
/// Tags stay in the body for display fidelity and are ALSO stored in the
/// tags table, which is what `--tag` queries hit.
pub fn parse_tags(body: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for word in body.split_whitespace() {
        let Some(rest) = word.strip_prefix('#') else {
            continue;
        };
        let tag: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect::<String>()
            .to_lowercase();
        if !tag.is_empty() && !out.contains(&tag) {
            out.push(tag);
        }
    }
    out
}

const NOTE_COLS: &str = "n.id, n.context_id, n.subpath, n.body, n.status, n.created_at, n.updated_at";

fn row_to_note(row: &Row<'_>, offset: usize) -> rusqlite::Result<Note> {
    Ok(Note {
        id: row.get(offset)?,
        context_id: row.get(offset + 1)?,
        subpath: row.get(offset + 2)?,
        body: row.get(offset + 3)?,
        status: Status::parse(&row.get::<_, String>(offset + 4)?),
        created_at: row.get(offset + 5)?,
        updated_at: row.get(offset + 6)?,
    })
}

fn limit_clause(limit: Option<usize>) -> String {
    match limit {
        Some(n) => format!(" LIMIT {n}"),
        None => String::new(),
    }
}

impl Store {
    pub fn add_note(
        &self,
        context_id: i64,
        subpath: &str,
        body: &str,
    ) -> Result<Note, StoreError> {
        let ts = now();
        self.conn()
            .execute(
                "INSERT INTO notes (context_id, subpath, body, status, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 'open', ?4, ?4)",
                params![context_id, subpath, body, ts],
            )
            .map_err(StoreError::Sqlite)?;
        let id = self.conn().last_insert_rowid();

        for tag in parse_tags(body) {
            self.conn()
                .execute("INSERT OR IGNORE INTO tags (name) VALUES (?1)", params![tag])
                .map_err(StoreError::Sqlite)?;
            let tag_id: i64 = self
                .conn()
                .query_row("SELECT id FROM tags WHERE name = ?1", params![tag], |r| r.get(0))
                .map_err(StoreError::Sqlite)?;
            self.conn()
                .execute(
                    "INSERT OR IGNORE INTO note_tags (note_id, tag_id) VALUES (?1, ?2)",
                    params![id, tag_id],
                )
                .map_err(StoreError::Sqlite)?;
        }

        Ok(Note {
            id,
            context_id,
            subpath: subpath.to_string(),
            body: body.to_string(),
            status: Status::Open,
            created_at: ts,
            updated_at: ts,
        })
    }

    pub fn list_notes(
        &self,
        context_id: i64,
        subpath: Option<&str>,
        include_done: bool,
        limit: Option<usize>,
    ) -> Result<Vec<Note>, StoreError> {
        let mut sql = format!("SELECT {NOTE_COLS} FROM notes n WHERE n.context_id = ?1");
        if !include_done {
            sql.push_str(" AND n.status = 'open'");
        }
        if subpath.is_some() {
            sql.push_str(" AND n.subpath = ?2");
        }
        sql.push_str(" ORDER BY n.created_at DESC, n.id DESC");
        sql.push_str(&limit_clause(limit));

        let conn = self.conn();
        let mut stmt = conn.prepare(&sql).map_err(StoreError::Sqlite)?;
        let map = |r: &Row<'_>| row_to_note(r, 0);
        let rows = match subpath {
            Some(sp) => stmt.query_map(params![context_id, sp], map),
            None => stmt.query_map(params![context_id], map),
        }
        .map_err(StoreError::Sqlite)?;

        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(StoreError::Sqlite)?);
        }
        Ok(out)
    }

    pub fn list_all_notes(
        &self,
        include_done: bool,
        limit: Option<usize>,
    ) -> Result<Vec<(Context, Note)>, StoreError> {
        let mut sql = format!(
            "SELECT {CTX_COLS}, {NOTE_COLS} FROM notes n
             JOIN contexts c ON c.id = n.context_id"
        );
        if !include_done {
            sql.push_str(" WHERE n.status = 'open'");
        }
        sql.push_str(" ORDER BY n.created_at DESC, n.id DESC");
        sql.push_str(&limit_clause(limit));

        let conn = self.conn();
        let mut stmt = conn.prepare(&sql).map_err(StoreError::Sqlite)?;
        let rows = stmt
            .query_map([], |r| Ok((row_to_context(r, 0)?, row_to_note(r, 7)?)))
            .map_err(StoreError::Sqlite)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(StoreError::Sqlite)?);
        }
        Ok(out)
    }

    pub fn search(
        &self,
        query: &str,
        context_id: Option<i64>,
        limit: Option<usize>,
    ) -> Result<Vec<(Context, Note)>, StoreError> {
        let mut sql = format!(
            "SELECT {CTX_COLS}, {NOTE_COLS} FROM notes_fts f
             JOIN notes n    ON n.id = f.rowid
             JOIN contexts c ON c.id = n.context_id
             WHERE notes_fts MATCH ?1"
        );
        if context_id.is_some() {
            sql.push_str(" AND n.context_id = ?2");
        }
        sql.push_str(" ORDER BY rank");
        sql.push_str(&limit_clause(limit));

        let conn = self.conn();
        let mut stmt = conn.prepare(&sql).map_err(StoreError::Sqlite)?;
        let map = |r: &Row<'_>| Ok((row_to_context(r, 0)?, row_to_note(r, 7)?));
        let rows = match context_id {
            Some(id) => stmt.query_map(params![query, id], map),
            None => stmt.query_map(params![query], map),
        }
        .map_err(StoreError::Sqlite)?;

        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(StoreError::Sqlite)?);
        }
        Ok(out)
    }

    pub fn notes_by_tag(
        &self,
        tag: &str,
        context_id: Option<i64>,
    ) -> Result<Vec<(Context, Note)>, StoreError> {
        let mut sql = format!(
            "SELECT {CTX_COLS}, {NOTE_COLS} FROM notes n
             JOIN contexts c  ON c.id = n.context_id
             JOIN note_tags nt ON nt.note_id = n.id
             JOIN tags t       ON t.id = nt.tag_id
             WHERE t.name = ?1"
        );
        if context_id.is_some() {
            sql.push_str(" AND n.context_id = ?2");
        }
        sql.push_str(" ORDER BY n.created_at DESC, n.id DESC");

        let conn = self.conn();
        let mut stmt = conn.prepare(&sql).map_err(StoreError::Sqlite)?;
        let map = |r: &Row<'_>| Ok((row_to_context(r, 0)?, row_to_note(r, 7)?));
        let lower = tag.to_lowercase();
        let rows = match context_id {
            Some(id) => stmt.query_map(params![lower, id], map),
            None => stmt.query_map(params![lower], map),
        }
        .map_err(StoreError::Sqlite)?;

        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(StoreError::Sqlite)?);
        }
        Ok(out)
    }

    pub fn set_status(&self, id: i64, status: Status) -> Result<bool, StoreError> {
        let n = self
            .conn()
            .execute(
                "UPDATE notes SET status = ?1, updated_at = ?2 WHERE id = ?3",
                params![status.as_str(), now(), id],
            )
            .map_err(StoreError::Sqlite)?;
        Ok(n > 0)
    }
}
