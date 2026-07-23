//! SQLite persistence layer for campaigns.

use rusqlite::Connection;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Campaign not found: {0}")]
    CampaignNotFound(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, StorageError>;

/// Initialize database schema
pub fn init_db(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS campaigns (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            dm_id TEXT NOT NULL,
            rule_set TEXT,
            created_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS members (
            campaign_id TEXT NOT NULL,
            player_id TEXT NOT NULL,
            role TEXT NOT NULL,
            joined_at INTEGER NOT NULL,
            PRIMARY KEY (campaign_id, player_id),
            FOREIGN KEY (campaign_id) REFERENCES campaigns(id)
        );

        CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            campaign_id TEXT NOT NULL,
            sequence_number INTEGER NOT NULL,
            event_type TEXT NOT NULL,
            author_id TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            payload TEXT NOT NULL,
            signature TEXT NOT NULL,
            FOREIGN KEY (campaign_id) REFERENCES campaigns(id)
        );

        CREATE INDEX IF NOT EXISTS idx_events_campaign ON events(campaign_id);
        CREATE INDEX IF NOT EXISTS idx_events_sequence ON events(campaign_id, sequence_number);

        CREATE TABLE IF NOT EXISTS player_data (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            campaign_id TEXT NOT NULL,
            owner_id TEXT NOT NULL,
            data_type TEXT NOT NULL,
            content TEXT NOT NULL,
            updated_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS world_data (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            campaign_id TEXT NOT NULL,
            owner_id TEXT NOT NULL,
            data_type TEXT NOT NULL,
            content TEXT NOT NULL,
            updated_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS plugin_state (
            plugin_name TEXT NOT NULL,
            campaign_id TEXT NOT NULL,
            version TEXT NOT NULL,
            state TEXT NOT NULL,
            PRIMARY KEY (plugin_name, campaign_id)
        );

        CREATE TABLE IF NOT EXISTS sync_peers (
            player_id TEXT PRIMARY KEY,
            last_sync_event_id INTEGER,
            last_sync_time INTEGER NOT NULL
        );
        "#,
    )?;

    Ok(())
}

/// Open or create campaign database
pub fn open_campaign_db<P: AsRef<Path>>(path: P) -> Result<Connection> {
    let conn = Connection::open(path)?;
    init_db(&conn)?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_db() {
        let conn = Connection::open_in_memory().unwrap();
        assert!(init_db(&conn).is_ok());
    }
}
