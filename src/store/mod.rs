pub mod contexts;
pub mod notes;
mod schema;

use std::path::{Path, PathBuf};

use rusqlite::Connection;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("sqlite error: {0}")]
    Sqlite(#[source] rusqlite::Error),
    #[error("migration to version {at} failed: {source}")]
    Migration {
        at: i64,
        #[source]
        source: rusqlite::Error,
    },
    #[error("could not determine home directory for the noteit database")]
    NoHome,
}

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(path: &Path) -> Result<Store, StoreError> {
        let mut conn = Connection::open(path).map_err(StoreError::Sqlite)?;
        Self::init(&mut conn)?;
        Ok(Store { conn })
    }

    pub fn open_in_memory() -> Result<Store, StoreError> {
        let mut conn = Connection::open_in_memory().map_err(StoreError::Sqlite)?;
        Self::init(&mut conn)?;
        Ok(Store { conn })
    }

    fn init(conn: &mut Connection) -> Result<(), StoreError> {
        // WAL + busy_timeout: two shells capturing at once is normal and
        // must never interleave into corruption.
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(StoreError::Sqlite)?;
        conn.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(StoreError::Sqlite)?;
        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(StoreError::Sqlite)?;
        schema::migrate(conn)
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}

/// Unix timestamp in seconds. THE single definition -- `contexts.rs` and
/// `notes.rs` both use this one; do not re-declare it per module.
pub(crate) fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// `%USERPROFILE%\noteit.db` on Windows, `$HOME/noteit.db` elsewhere.
pub fn default_db_path() -> Result<PathBuf, StoreError> {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .ok_or(StoreError::NoHome)?;
    Ok(PathBuf::from(home).join("noteit.db"))
}
