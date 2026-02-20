//! SQLite persistence layer for the MEMZ memory system.
//!
//! Each entity's [`MemoryBank`] is serialised to JSON and stored in a
//! per-world SQLite database.  The schema is intentionally simple:
//!
//! ```sql
//! CREATE TABLE IF NOT EXISTS memory_banks (
//!     entity_id  TEXT PRIMARY KEY,
//!     data       BLOB NOT NULL,
//!     updated_at TEXT NOT NULL,
//!     checksum   TEXT
//! );
//! ```
//!
//! Design rationale (from §12 of the design doc):
//! - WAL mode for concurrent reads during gameplay
//! - JSON inside a BLOB column keeps the schema stable across memory-type
//!   changes (forward-compatible).
//! - Optional CRC-32 checksum detects save corruption.
//! - Backup support via SQLite's online-backup API.

use std::path::{Path, PathBuf};
use std::time::Instant;

use chrono::Utc;
use rusqlite::{params, Connection, OpenFlags};
use tracing::{debug, info, warn};

use crate::config::PersistenceConfig;
use crate::error::{MemzError, Result};
use crate::memory::MemoryBank;
use crate::types::EntityId;

// ---------------------------------------------------------------------------
// CRC-32 checksum helper
// ---------------------------------------------------------------------------

/// Compute a CRC-32 checksum (Castagnoli / CRC-32C) of `data` and return it
/// as a lowercase hex string.
fn crc32_hex(data: &[u8]) -> String {
    // Simple CRC-32 using the standard ISO polynomial via a lookup table.
    // We avoid pulling in an extra crate by using a basic implementation.
    let crc = crc32_compute(data);
    format!("{crc:08x}")
}

/// Basic CRC-32 (ISO 3309 / ITU-T V.42) computation.
fn crc32_compute(data: &[u8]) -> u32 {
    const POLY: u32 = 0xEDB8_8320;
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            if crc & 1 == 1 {
                crc = (crc >> 1) ^ POLY;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

// ---------------------------------------------------------------------------
// PersistenceEngine
// ---------------------------------------------------------------------------

/// Handle to an open SQLite database that stores [`MemoryBank`]s.
///
/// # Usage
///
/// ```no_run
/// # use memz_core::persistence::PersistenceEngine;
/// # use memz_core::config::PersistenceConfig;
/// # use memz_core::types::EntityId;
/// # use memz_core::memory::MemoryBank;
/// let engine = PersistenceEngine::open("world_save.db", &PersistenceConfig::default())?;
/// let entity = EntityId::new();
/// let bank = MemoryBank::new();
/// engine.save_bank(&entity, &bank)?;
/// let loaded = engine.load_bank(&entity)?;
/// # Ok::<(), memz_core::error::MemzError>(())
/// ```
pub struct PersistenceEngine {
    conn: Connection,
    config: PersistenceConfig,
    db_path: PathBuf,
}

impl std::fmt::Debug for PersistenceEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PersistenceEngine")
            .field("db_path", &self.db_path)
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl PersistenceEngine {
    /// Open (or create) an SQLite database at `path`.
    ///
    /// The schema is automatically created if it does not exist.
    /// WAL mode is enabled when `config.wal_mode` is `true`.
    ///
    /// # Errors
    ///
    /// Returns [`MemzError::Database`] on SQLite failures.
    pub fn open<P: AsRef<Path>>(path: P, config: &PersistenceConfig) -> Result<Self> {
        let db_path = path.as_ref().to_path_buf();
        let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX;

        let conn = Connection::open_with_flags(&db_path, flags)?;

        // Pragmas for performance and safety.
        if config.wal_mode {
            conn.execute_batch("PRAGMA journal_mode = WAL;")?;
        }
        conn.execute_batch("PRAGMA synchronous = NORMAL;")?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        conn.execute_batch("PRAGMA busy_timeout = 5000;")?;

        // Schema creation.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS memory_banks (
                entity_id  TEXT PRIMARY KEY,
                data       BLOB NOT NULL,
                updated_at TEXT NOT NULL,
                checksum   TEXT
            );",
        )?;

        info!(
            path = %db_path.display(),
            wal = config.wal_mode,
            "MEMZ persistence engine opened"
        );

        Ok(Self {
            conn,
            config: config.clone(),
            db_path,
        })
    }

    /// Open an in-memory database (useful for tests).
    ///
    /// # Errors
    ///
    /// Returns [`MemzError::Database`] on SQLite failures.
    pub fn open_in_memory(config: &PersistenceConfig) -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS memory_banks (
                entity_id  TEXT PRIMARY KEY,
                data       BLOB NOT NULL,
                updated_at TEXT NOT NULL,
                checksum   TEXT
            );",
        )?;

        Ok(Self {
            conn,
            config: config.clone(),
            db_path: PathBuf::from(":memory:"),
        })
    }

    // ------------------------------------------------------------------
    // Core CRUD
    // ------------------------------------------------------------------

    /// Save (upsert) an entity's [`MemoryBank`].
    ///
    /// The bank is serialised to JSON.  If `config.checksum_enabled` is true,
    /// a CRC-32 of the JSON bytes is stored alongside the data.
    ///
    /// # Errors
    ///
    /// Returns [`MemzError::Serialization`] if JSON encoding fails, or
    /// [`MemzError::Database`] on SQLite failures.
    pub fn save_bank(&self, entity_id: &EntityId, bank: &MemoryBank) -> Result<()> {
        let start = Instant::now();

        let json = serde_json::to_vec(bank).map_err(|e| MemzError::Serialization(e.to_string()))?;

        let checksum = if self.config.checksum_enabled {
            Some(crc32_hex(&json))
        } else {
            None
        };

        let now = Utc::now().to_rfc3339();
        let id_str = entity_id.0.to_string();

        self.conn.execute(
            "INSERT INTO memory_banks (entity_id, data, updated_at, checksum)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(entity_id) DO UPDATE SET
                data = excluded.data,
                updated_at = excluded.updated_at,
                checksum = excluded.checksum",
            params![id_str, json, now, checksum],
        )?;

        let elapsed = start.elapsed();
        debug!(
            entity = %entity_id,
            memories = bank.total_count(),
            bytes = json.len(),
            elapsed_us = elapsed.as_micros(),
            "Saved memory bank"
        );

        Ok(())
    }

    /// Load an entity's [`MemoryBank`].
    ///
    /// Returns `None` if no row exists for the given entity.
    /// If checksums are enabled and the stored checksum doesn't match, a
    /// warning is logged but the data is still returned.
    ///
    /// # Errors
    ///
    /// Returns [`MemzError::Serialization`] if JSON decoding fails, or
    /// [`MemzError::Database`] on SQLite failures.
    pub fn load_bank(&self, entity_id: &EntityId) -> Result<Option<MemoryBank>> {
        let start = Instant::now();
        let id_str = entity_id.0.to_string();

        let mut stmt = self
            .conn
            .prepare_cached("SELECT data, checksum FROM memory_banks WHERE entity_id = ?1")?;

        let result: Option<(Vec<u8>, Option<String>)> = stmt
            .query_row(params![id_str], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .optional()?;

        let Some((data, stored_checksum)) = result else {
            return Ok(None);
        };

        // Verify checksum if enabled.
        if self.config.checksum_enabled {
            if let Some(ref expected) = stored_checksum {
                let actual = crc32_hex(&data);
                if *expected != actual {
                    warn!(
                        entity = %entity_id,
                        expected = %expected,
                        actual = %actual,
                        "Checksum mismatch — possible save corruption"
                    );
                }
            }
        }

        let bank: MemoryBank =
            serde_json::from_slice(&data).map_err(|e| MemzError::Serialization(e.to_string()))?;

        let elapsed = start.elapsed();
        debug!(
            entity = %entity_id,
            memories = bank.total_count(),
            elapsed_us = elapsed.as_micros(),
            "Loaded memory bank"
        );

        Ok(Some(bank))
    }

    /// Delete an entity's [`MemoryBank`].
    ///
    /// Returns `true` if a row was actually deleted.
    ///
    /// # Errors
    ///
    /// Returns [`MemzError::Database`] on SQLite failures.
    pub fn delete_bank(&self, entity_id: &EntityId) -> Result<bool> {
        let id_str = entity_id.0.to_string();
        let deleted = self
            .conn
            .execute("DELETE FROM memory_banks WHERE entity_id = ?1", params![id_str])?;
        Ok(deleted > 0)
    }

    /// List all entity IDs that have a saved [`MemoryBank`].
    ///
    /// # Errors
    ///
    /// Returns [`MemzError::Database`] on SQLite failures.
    pub fn list_entities(&self) -> Result<Vec<EntityId>> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT entity_id FROM memory_banks")?;

        let rows = stmt.query_map([], |row| {
            let id_str: String = row.get(0)?;
            Ok(id_str)
        })?;

        let mut entities = Vec::new();
        for row in rows {
            let id_str = row?;
            if let Ok(uuid) = uuid::Uuid::parse_str(&id_str) {
                entities.push(EntityId(uuid));
            } else {
                warn!(id = %id_str, "Skipping row with invalid UUID");
            }
        }

        Ok(entities)
    }

    /// Return the total number of stored entities.
    ///
    /// # Errors
    ///
    /// Returns [`MemzError::Database`] on SQLite failures.
    pub fn entity_count(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM memory_banks", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    // ------------------------------------------------------------------
    // Backup
    // ------------------------------------------------------------------

    /// Create a backup of the database to `dest_path` using SQLite's
    /// online-backup API.
    ///
    /// This is safe to call while the database is being read/written.
    ///
    /// # Errors
    ///
    /// Returns [`MemzError::Database`] on SQLite failures, or
    /// [`MemzError::Io`] if the destination is not writable.
    pub fn backup<P: AsRef<Path>>(&self, dest_path: P) -> Result<()> {
        let start = Instant::now();
        let mut dest = Connection::open(dest_path.as_ref())?;
        let backup = rusqlite::backup::Backup::new(&self.conn, &mut dest)?;

        // Step through 256 pages at a time, sleeping 50ms between steps.
        backup.run_to_completion(256, std::time::Duration::from_millis(50), None)?;

        info!(
            dest = %dest_path.as_ref().display(),
            elapsed_ms = start.elapsed().as_millis(),
            "Database backup completed"
        );
        Ok(())
    }

    /// Create a numbered backup alongside the database file, rotating old
    /// backups so that at most `config.backup_count` are kept.
    ///
    /// # Errors
    ///
    /// Returns [`MemzError::Database`] or [`MemzError::Io`] on failure.
    pub fn create_rotating_backup(&self) -> Result<()> {
        if self.db_path.as_os_str() == ":memory:" {
            return Ok(());
        }

        let max = self.config.backup_count;
        if max == 0 {
            return Ok(());
        }

        // Rotate existing backups (highest first so we don't overwrite).
        for i in (1..max).rev() {
            let src = self.backup_path(i);
            let dst = self.backup_path(i + 1);
            if src.exists() {
                std::fs::rename(&src, &dst)?;
            }
        }

        // Remove the oldest if it now exceeds the limit.
        let oldest = self.backup_path(max + 1);
        if oldest.exists() {
            std::fs::remove_file(&oldest)?;
        }

        // Create the fresh backup as backup.1
        let dest = self.backup_path(1);
        self.backup(&dest)?;

        info!(
            max_backups = max,
            "Rotating backup created"
        );

        Ok(())
    }

    /// Path to a numbered backup file (e.g. `world_save.db.bak.1`).
    fn backup_path(&self, n: u32) -> PathBuf {
        let mut p = self.db_path.clone();
        let ext = format!(
            "{}.bak.{n}",
            p.extension()
                .map_or(String::new(), |e| e.to_string_lossy().into_owned())
        );
        p.set_extension(ext);
        p
    }

    // ------------------------------------------------------------------
    // Utility
    // ------------------------------------------------------------------

    /// Return the path to the database file (or `:memory:` for in-memory DBs).
    #[must_use]
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    /// Run an integrity check on the database.
    ///
    /// Returns `Ok(true)` if the database passes the check, `Ok(false)` if
    /// corruption is detected.
    ///
    /// # Errors
    ///
    /// Returns [`MemzError::Database`] if the integrity check query itself fails.
    pub fn integrity_check(&self) -> Result<bool> {
        let result: String =
            self.conn
                .query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
        Ok(result == "ok")
    }

    /// Reclaim unused space by running `VACUUM`.
    ///
    /// # Errors
    ///
    /// Returns [`MemzError::Database`] on SQLite failures.
    pub fn vacuum(&self) -> Result<()> {
        self.conn.execute_batch("VACUUM;")?;
        Ok(())
    }
}

/// Extension trait that adds an `.optional()` combinator to `rusqlite::Result`.
///
/// Converts `Err(QueryReturnedNoRows)` into `Ok(None)`.
trait OptionalExt<T> {
    /// Convert `QueryReturnedNoRows` into `Ok(None)`.
    fn optional(self) -> std::result::Result<Option<T>, rusqlite::Error>;
}

impl<T> OptionalExt<T> for std::result::Result<T, rusqlite::Error> {
    fn optional(self) -> std::result::Result<Option<T>, rusqlite::Error> {
        match self {
            Ok(val) => Ok(Some(val)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::episodic::EpisodicMemory;
    use crate::memory::social::SocialMemory;
    use crate::types::{EntityId, GameTimestamp, Location, MemoryId};
    use chrono::Utc;

    fn test_config() -> PersistenceConfig {
        PersistenceConfig {
            checksum_enabled: true,
            ..PersistenceConfig::default()
        }
    }

    fn sample_bank() -> MemoryBank {
        let mut bank = MemoryBank::new();
        bank.episodic.push(EpisodicMemory {
            id: MemoryId::new(),
            event: "Met a wandering bard at the tavern".to_string(),
            participants: vec![EntityId::new()],
            location: Location {
                x: 100.0,
                y: 50.0,
                z: 0.0,
            },
            timestamp: GameTimestamp {
                tick: 1000,
                real_time: Utc::now(),
            },
            emotional_valence: 0.6,
            importance: 0.7,
            decay_rate: 0.02,
            strength: 1.0,
            access_count: 0,
            last_accessed: GameTimestamp {
                tick: 1000,
                real_time: Utc::now(),
            },
            is_first_meeting: true,
            embedding: None,
        });
        bank.social.push(SocialMemory {
            id: MemoryId::new(),
            about: EntityId::new(),
            source: EntityId::new(),
            claim: "The blacksmith is secretly a mage".to_string(),
            believed: true,
            disbelief_reason: None,
            trust_in_source: 0.8,
            propagation_depth: 1,
            received_at: GameTimestamp {
                tick: 1001,
                real_time: Utc::now(),
            },
            sentiment: 0.3,
        });
        bank
    }

    #[test]
    fn round_trip_save_load() {
        let engine = PersistenceEngine::open_in_memory(&test_config()).expect("open");
        let entity = EntityId::new();
        let bank = sample_bank();

        engine.save_bank(&entity, &bank).expect("save");
        let loaded = engine.load_bank(&entity).expect("load").expect("Some");

        assert_eq!(loaded.episodic.len(), 1);
        assert_eq!(loaded.social.len(), 1);
        assert_eq!(loaded.episodic[0].event, bank.episodic[0].event);
        assert_eq!(loaded.social[0].claim, bank.social[0].claim);
    }

    #[test]
    fn load_nonexistent_returns_none() {
        let engine = PersistenceEngine::open_in_memory(&test_config()).expect("open");
        let entity = EntityId::new();
        let result = engine.load_bank(&entity).expect("load");
        assert!(result.is_none());
    }

    #[test]
    fn upsert_overwrites() {
        let engine = PersistenceEngine::open_in_memory(&test_config()).expect("open");
        let entity = EntityId::new();

        let bank1 = sample_bank();
        engine.save_bank(&entity, &bank1).expect("save1");

        let mut bank2 = sample_bank();
        bank2.episodic.push(bank2.episodic[0].clone());
        engine.save_bank(&entity, &bank2).expect("save2");

        let loaded = engine.load_bank(&entity).expect("load").expect("Some");
        assert_eq!(loaded.episodic.len(), 2, "Should reflect the second save");
    }

    #[test]
    fn delete_bank_works() {
        let engine = PersistenceEngine::open_in_memory(&test_config()).expect("open");
        let entity = EntityId::new();
        let bank = sample_bank();

        engine.save_bank(&entity, &bank).expect("save");
        assert!(engine.delete_bank(&entity).expect("delete"));
        assert!(!engine.delete_bank(&entity).expect("delete again"));
        assert!(engine.load_bank(&entity).expect("load").is_none());
    }

    #[test]
    fn list_entities_and_count() {
        let engine = PersistenceEngine::open_in_memory(&test_config()).expect("open");

        let e1 = EntityId::new();
        let e2 = EntityId::new();
        let e3 = EntityId::new();
        let bank = MemoryBank::new();

        engine.save_bank(&e1, &bank).expect("save");
        engine.save_bank(&e2, &bank).expect("save");
        engine.save_bank(&e3, &bank).expect("save");

        let entities = engine.list_entities().expect("list");
        assert_eq!(entities.len(), 3);
        assert_eq!(engine.entity_count().expect("count"), 3);
    }

    #[test]
    fn integrity_check_passes() {
        let engine = PersistenceEngine::open_in_memory(&test_config()).expect("open");
        assert!(engine.integrity_check().expect("check"));
    }

    #[test]
    fn checksum_detection() {
        // Save with checksums, then manually corrupt and reload to verify
        // the warning path. We can't easily assert on tracing output, so we
        // just ensure the load still succeeds (warnings are logged).
        let engine = PersistenceEngine::open_in_memory(&test_config()).expect("open");
        let entity = EntityId::new();
        let bank = sample_bank();
        engine.save_bank(&entity, &bank).expect("save");

        // Manually overwrite the checksum with a wrong value.
        let id_str = entity.0.to_string();
        engine
            .conn
            .execute(
                "UPDATE memory_banks SET checksum = 'deadbeef' WHERE entity_id = ?1",
                params![id_str],
            )
            .expect("corrupt checksum");

        // Load should still work but would have logged a warning.
        let loaded = engine.load_bank(&entity).expect("load").expect("Some");
        assert_eq!(loaded.episodic.len(), 1);
    }

    #[test]
    fn file_based_open_and_backup() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("test_memz.db");
        let config = test_config();

        let engine = PersistenceEngine::open(&db_path, &config).expect("open");
        let entity = EntityId::new();
        engine
            .save_bank(&entity, &sample_bank())
            .expect("save");

        // Backup to a second file.
        let backup_path = dir.path().join("test_memz_backup.db");
        engine.backup(&backup_path).expect("backup");

        // Open the backup and verify data.
        let backup_engine = PersistenceEngine::open(&backup_path, &config).expect("open backup");
        let loaded = backup_engine
            .load_bank(&entity)
            .expect("load from backup")
            .expect("Some");
        assert_eq!(loaded.episodic.len(), 1);
    }

    #[test]
    fn rotating_backup() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("world.db");
        let mut config = test_config();
        config.backup_count = 2;

        let engine = PersistenceEngine::open(&db_path, &config).expect("open");
        engine
            .save_bank(&EntityId::new(), &sample_bank())
            .expect("save");

        // Create 3 backups, should keep at most 2.
        engine.create_rotating_backup().expect("backup 1");
        engine.create_rotating_backup().expect("backup 2");
        engine.create_rotating_backup().expect("backup 3");

        assert!(dir.path().join("world.db.bak.1").exists());
        assert!(dir.path().join("world.db.bak.2").exists());
        // The 3rd oldest should have been removed.
        assert!(!dir.path().join("world.db.bak.3").exists());
    }

    #[test]
    fn crc32_basic() {
        // Known test vector: CRC-32 of "123456789" = 0xCBF43926
        let crc = crc32_compute(b"123456789");
        assert_eq!(crc, 0xCBF4_3926);
    }
}
