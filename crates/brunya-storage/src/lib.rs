//! Camada de persistência local SQLite do Brunya Browser.
//!
//! Gerencia histórico, favoritos, configurações e permissões por site,
//! com migrações de esquema versionadas e proteção contra corrupção.

use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Erro SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("Erro de migração de esquema: versão atual {current}, solicitada {target}")]
    MigrationFailed { current: i32, target: i32 },

    #[error("Erro de serialização JSON: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, StorageError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryItem {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub visit_count: i32,
    pub last_visit: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkItem {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub folder: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionState {
    Ask,
    Allow,
    Block,
}

impl PermissionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            PermissionState::Ask => "ask",
            PermissionState::Allow => "allow",
            PermissionState::Block => "block",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "allow" => PermissionState::Allow,
            "block" => PermissionState::Block,
            _ => PermissionState::Ask,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DownloadState {
    InProgress,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

impl DownloadState {
    pub fn as_str(&self) -> &'static str {
        match self {
            DownloadState::InProgress => "in_progress",
            DownloadState::Paused => "paused",
            DownloadState::Completed => "completed",
            DownloadState::Failed => "failed",
            DownloadState::Cancelled => "cancelled",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "paused" => DownloadState::Paused,
            "completed" => DownloadState::Completed,
            "failed" => DownloadState::Failed,
            "cancelled" => DownloadState::Cancelled,
            _ => DownloadState::InProgress,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadItem {
    pub id: i64,
    pub url: String,
    pub filename: String,
    pub target_path: String,
    pub total_bytes: i64,
    pub downloaded_bytes: i64,
    pub state: DownloadState,
    pub created_at: DateTime<Utc>,
}

impl DownloadItem {
    pub fn format_size(bytes: i64) -> String {
        if bytes < 1024 {
            format!("{} B", bytes)
        } else if bytes < 1024 * 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else if bytes < 1024 * 1024 * 1024 {
            format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
        } else {
            format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
        }
    }

    pub fn progress_percent(&self) -> f64 {
        if self.total_bytes <= 0 {
            0.0
        } else {
            ((self.downloaded_bytes as f64 / self.total_bytes as f64) * 100.0).clamp(0.0, 100.0)
        }
    }
}

pub struct StorageManager {
    conn: Mutex<Connection>,
}

impl StorageManager {
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let mgr = Self {
            conn: Mutex::new(conn),
        };
        mgr.migrate()?;
        Ok(mgr)
    }

    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let conn = Connection::open(path)?;
        let mgr = Self {
            conn: Mutex::new(conn),
        };
        mgr.migrate()?;
        Ok(mgr)
    }

    fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock();

        // Tabela de controle de versão
        conn.execute(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        let current_version: i32 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if current_version < 1 {
            info!("Aplicando migração v1 do banco de dados Brunya...");
            conn.execute_batch(
                "BEGIN;
                CREATE TABLE IF NOT EXISTS history (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    url TEXT NOT NULL UNIQUE,
                    title TEXT NOT NULL,
                    visit_count INTEGER DEFAULT 1,
                    last_visit INTEGER NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_history_last_visit ON history(last_visit DESC);

                CREATE TABLE IF NOT EXISTS bookmarks (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    url TEXT NOT NULL,
                    title TEXT NOT NULL,
                    folder TEXT NOT NULL DEFAULT '',
                    created_at INTEGER NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_bookmarks_url ON bookmarks(url);

                CREATE TABLE IF NOT EXISTS site_permissions (
                    origin TEXT NOT NULL,
                    permission TEXT NOT NULL,
                    state TEXT NOT NULL,
                    PRIMARY KEY (origin, permission)
                );

                CREATE TABLE IF NOT EXISTS settings_kv (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );

                INSERT INTO schema_migrations (version) VALUES (1);
                COMMIT;",
            )?;
        }

        if current_version < 2 {
            info!("Aplicando migração v2 do banco de dados Brunya (tabela de downloads)...");
            conn.execute_batch(
                "BEGIN;
                CREATE TABLE IF NOT EXISTS downloads (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    url TEXT NOT NULL,
                    filename TEXT NOT NULL,
                    target_path TEXT NOT NULL,
                    total_bytes INTEGER NOT NULL,
                    downloaded_bytes INTEGER NOT NULL,
                    state TEXT NOT NULL,
                    created_at INTEGER NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_downloads_created ON downloads(created_at DESC);
                INSERT INTO schema_migrations (version) VALUES (2);
                COMMIT;",
            )?;
        }

        Ok(())
    }

    // --- Histórico ---

    pub fn record_history_visit(&self, url: &str, title: &str) -> Result<()> {
        let conn = self.conn.lock();
        let now_ts = Utc::now().timestamp();

        conn.execute(
            "INSERT INTO history (url, title, visit_count, last_visit)
             VALUES (?1, ?2, 1, ?3)
             ON CONFLICT(url) DO UPDATE SET
                title = CASE WHEN ?2 != '' THEN ?2 ELSE title END,
                visit_count = visit_count + 1,
                last_visit = ?3",
            params![url, title, now_ts],
        )?;

        Ok(())
    }

    pub fn get_recent_history(&self, limit: usize) -> Result<Vec<HistoryItem>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, url, title, visit_count, last_visit
             FROM history
             ORDER BY last_visit DESC, id DESC
             LIMIT ?1",
        )?;

        let rows = stmt.query_map(params![limit as i64], |row| {
            let ts: i64 = row.get(4)?;
            let dt = DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now);
            Ok(HistoryItem {
                id: row.get(0)?,
                url: row.get(1)?,
                title: row.get(2)?,
                visit_count: row.get(3)?,
                last_visit: dt,
            })
        })?;

        let mut items = Vec::new();
        for r in rows {
            items.push(r?);
        }
        Ok(items)
    }

    pub fn clear_history(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM history", [])?;
        Ok(())
    }

    // --- Favoritos ---

    pub fn add_bookmark(&self, url: &str, title: &str, folder: &str) -> Result<i64> {
        let conn = self.conn.lock();
        let now_ts = Utc::now().timestamp();
        conn.execute(
            "INSERT INTO bookmarks (url, title, folder, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![url, title, folder, now_ts],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_bookmarks(&self) -> Result<Vec<BookmarkItem>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, url, title, folder, created_at FROM bookmarks ORDER BY id DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            let ts: i64 = row.get(4)?;
            let dt = DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now);
            Ok(BookmarkItem {
                id: row.get(0)?,
                url: row.get(1)?,
                title: row.get(2)?,
                folder: row.get(3)?,
                created_at: dt,
            })
        })?;

        let mut items = Vec::new();
        for r in rows {
            items.push(r?);
        }
        Ok(items)
    }

    pub fn delete_bookmark(&self, id: i64) -> Result<bool> {
        let conn = self.conn.lock();
        let rows = conn.execute("DELETE FROM bookmarks WHERE id = ?1", params![id])?;
        Ok(rows > 0)
    }

    // --- Downloads ---

    pub fn add_download(
        &self,
        url: &str,
        filename: &str,
        target_path: &str,
        total_bytes: i64,
    ) -> Result<i64> {
        let conn = self.conn.lock();
        let now_ts = Utc::now().timestamp();
        conn.execute(
            "INSERT INTO downloads (url, filename, target_path, total_bytes, downloaded_bytes, state, created_at)
             VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6)",
            params![url, filename, target_path, total_bytes, DownloadState::InProgress.as_str(), now_ts],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn update_download_progress(&self, id: i64, downloaded: i64, state: DownloadState) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE downloads SET downloaded_bytes = ?1, state = ?2 WHERE id = ?3",
            params![downloaded, state.as_str(), id],
        )?;
        Ok(())
    }

    pub fn list_downloads(&self, limit: usize) -> Result<Vec<DownloadItem>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, url, filename, target_path, total_bytes, downloaded_bytes, state, created_at
             FROM downloads
             ORDER BY created_at DESC, id DESC
             LIMIT ?1",
        )?;

        let rows = stmt.query_map(params![limit as i64], |row| {
            let state_str: String = row.get(6)?;
            let ts: i64 = row.get(7)?;
            let dt = DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now);
            Ok(DownloadItem {
                id: row.get(0)?,
                url: row.get(1)?,
                filename: row.get(2)?,
                target_path: row.get(3)?,
                total_bytes: row.get(4)?,
                downloaded_bytes: row.get(5)?,
                state: DownloadState::from_str(&state_str),
                created_at: dt,
            })
        })?;

        let mut items = Vec::new();
        for r in rows {
            items.push(r?);
        }
        Ok(items)
    }

    pub fn clear_completed_downloads(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM downloads WHERE state IN ('completed', 'cancelled', 'failed')",
            [],
        )?;
        Ok(())
    }

    // --- Permissões por Site ---

    pub fn set_site_permission(&self, origin: &str, permission: &str, state: PermissionState) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO site_permissions (origin, permission, state)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(origin, permission) DO UPDATE SET state = ?3",
            params![origin, permission, state.as_str()],
        )?;
        Ok(())
    }

    pub fn get_site_permission(&self, origin: &str, permission: &str) -> Result<PermissionState> {
        let conn = self.conn.lock();
        let state_str: Option<String> = conn
            .query_row(
                "SELECT state FROM site_permissions WHERE origin = ?1 AND permission = ?2",
                params![origin, permission],
                |row| row.get(0),
            )
            .optional()?;

        Ok(match state_str {
            Some(s) => PermissionState::from_str(&s),
            None => PermissionState::Ask,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_history_and_bookmarks() {
        let db = StorageManager::open_in_memory().expect("Falha ao abrir SQLite em memória");

        // Histórico
        db.record_history_visit("https://linuxmint.com", "Linux Mint Home").unwrap();
        db.record_history_visit("https://linuxmint.com", "Linux Mint Home").unwrap();
        db.record_history_visit("https://rust-lang.org", "Rust").unwrap();

        let hist = db.get_recent_history(10).unwrap();
        assert_eq!(hist.len(), 2);
        let urls: Vec<String> = hist.iter().map(|h| h.url.clone()).collect();
        assert!(urls.contains(&"https://rust-lang.org".to_string()));
        assert!(urls.contains(&"https://linuxmint.com".to_string()));
        let mint = hist.iter().find(|h| h.url == "https://linuxmint.com").unwrap();
        assert_eq!(mint.visit_count, 2);

        // Favoritos
        let bm_id = db.add_bookmark("https://kernel.org", "Linux Kernel", "Tech").unwrap();
        let bookmarks = db.list_bookmarks().unwrap();
        assert_eq!(bookmarks.len(), 1);
        assert_eq!(bookmarks[0].url, "https://kernel.org");

        assert!(db.delete_bookmark(bm_id).unwrap());
        assert_eq!(db.list_bookmarks().unwrap().len(), 0);

        // Downloads
        let dl_id = db.add_download(
            "https://kernel.org/linux-6.12.tar.xz",
            "linux-6.12.tar.xz",
            "/home/user/Downloads/linux-6.12.tar.xz",
            145000000,
        ).unwrap();

        db.update_download_progress(dl_id, 145000000, DownloadState::Completed).unwrap();
        let downloads = db.list_downloads(10).unwrap();
        assert_eq!(downloads.len(), 1);
        assert_eq!(downloads[0].state, DownloadState::Completed);
        assert_eq!(downloads[0].progress_percent(), 100.0);

        db.clear_completed_downloads().unwrap();
        assert_eq!(db.list_downloads(10).unwrap().len(), 0);

        // Permissões
        db.set_site_permission("https://meet.jit.si", "camera", PermissionState::Allow).unwrap();
        assert_eq!(
            db.get_site_permission("https://meet.jit.si", "camera").unwrap(),
            PermissionState::Allow
        );
        assert_eq!(
            db.get_site_permission("https://meet.jit.si", "microphone").unwrap(),
            PermissionState::Ask
        );
    }
}
