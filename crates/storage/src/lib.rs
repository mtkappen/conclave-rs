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

    #[error("JSON error: {0}")]
    JsonError(serde_json::Error),

    #[error("UUID parse error: {0}")]
    UuidError(uuid::Error),
}

impl From<serde_json::Error> for StorageError {
    fn from(err: serde_json::Error) -> Self {
        StorageError::JsonError(err)
    }
}

impl From<uuid::Error> for StorageError {
    fn from(err: uuid::Error) -> Self {
        StorageError::UuidError(err)
    }
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

/// Store a signed event in the database
pub fn store_event(conn: &Connection, event: &conclave_protocol::SignedEvent) -> Result<()> {
    conn.execute(
        "INSERT INTO events (id, campaign_id, sequence_number, event_type, author_id, timestamp, payload, signature) 
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            event.id as i64,
            event.campaign_id.to_string(),
            event.sequence_number as i64,
            event.event_type,
            event.author_id,
            event.timestamp.duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_secs() as i64,
            serde_json::to_string(&event.payload)?,
            event.signature,
        ],
    )?;
    Ok(())
}

type EventRow = (i64, String, i64, String, String, i64, String, String);

/// Get all events for a campaign up to a sequence number
pub fn get_events_up_to(conn: &Connection, campaign_id: uuid::Uuid, max_sequence: u64) -> Result<Vec<conclave_protocol::SignedEvent>> {
    let mut stmt = conn.prepare(
        "SELECT id, campaign_id, sequence_number, event_type, author_id, timestamp, payload, signature 
         FROM events WHERE campaign_id = ?1 AND sequence_number <= ?2 ORDER BY sequence_number"
    )?;
    
    let raw_rows: Result<Vec<EventRow>> = stmt.query_map(
        rusqlite::params![campaign_id.to_string(), max_sequence as i64],
        |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
            ))
        },
    )?.map(|r| r.map_err(StorageError::Database)).collect();
    
    let mut events = Vec::new();
    for row in raw_rows? {
        events.push(conclave_protocol::SignedEvent {
            id: row.0 as u64,
            campaign_id: row.1.parse()?,
            sequence_number: row.2 as u64,
            event_type: row.3,
            author_id: row.4,
            timestamp: std::time::SystemTime::UNIX_EPOCH + 
                std::time::Duration::from_secs(row.5 as u64),
            payload: serde_json::from_str(&row.6)?,
            signature: row.7,
        });
    }
    
    Ok(events)
}

/// Get the highest sequence number for a campaign
pub fn get_max_sequence(conn: &Connection, campaign_id: uuid::Uuid) -> Result<u64> {
    let max_seq: Option<i64> = conn.query_row(
        "SELECT MAX(sequence_number) FROM events WHERE campaign_id = ?1",
        rusqlite::params![campaign_id.to_string()],
        |row| row.get(0),
    )?;
    
    Ok(max_seq.unwrap_or(0) as u64)
}

/// Store a campaign member
pub fn add_member(conn: &Connection, campaign_id: uuid::Uuid, player_id: String, role: String) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO members (campaign_id, player_id, role, joined_at) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![
            campaign_id.to_string(),
            player_id,
            role,
            chrono::Utc::now().timestamp()
        ],
    )?;
    Ok(())
}

/// Get all members of a campaign
pub fn get_members(conn: &Connection, campaign_id: uuid::Uuid) -> Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT player_id, role FROM members WHERE campaign_id = ?1"
    )?;
    
    let rows = stmt.query_map(
        rusqlite::params![campaign_id.to_string()],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    )?;
    
    rows.map(|r| r.map_err(StorageError::Database)).collect()
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
