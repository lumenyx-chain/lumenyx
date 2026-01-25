//! Pool Mode Handle - Runtime toggle SOLO↔POOL with persistence
//!
//! This module provides:
//! - Runtime toggle between SOLO and POOL mining modes
//! - Persistence to disk so mode survives restarts
//! - Thread-safe access via tokio::sync::watch

use std::{
    fs,
    io,
    path::PathBuf,
    sync::Arc,
};

use tokio::sync::watch;

/// Get the data directory path: ~/.local/share/lumenyx-node
pub fn data_dir() -> io::Result<PathBuf> {
    let mut base = dirs::data_local_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "data_local_dir not found"))?;
    base.push("lumenyx-node");
    Ok(base)
}

/// Get the pool_mode.conf file path
pub fn pool_mode_path() -> io::Result<PathBuf> {
    Ok(data_dir()?.join("pool_mode.conf"))
}

/// Read persisted pool mode from disk
/// Returns None if file doesn't exist (use CLI default)
pub fn read_persisted_pool_mode() -> io::Result<Option<bool>> {
    let p = pool_mode_path()?;
    if !p.exists() {
        return Ok(None);
    }
    let s = fs::read_to_string(&p)?;
    let s = s.trim().to_ascii_lowercase();
    match s.as_str() {
        "1" | "true" | "pool" => Ok(Some(true)),
        "0" | "false" | "solo" => Ok(Some(false)),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid pool_mode.conf contents: {s}"),
        )),
    }
}

/// Write pool mode to disk for persistence across restarts
pub fn write_persisted_pool_mode(enabled: bool) -> io::Result<()> {
    let dir = data_dir()?;
    fs::create_dir_all(&dir)?;
    let p = dir.join("pool_mode.conf");
    fs::write(p, if enabled { "true\n" } else { "false\n" })?;
    Ok(())
}

/// Shared pool mode handle type
pub type SharedPoolMode = Arc<PoolModeHandle>;

/// Handle for runtime pool mode toggle
/// Uses tokio::sync::watch for efficient notifications
#[derive(Debug)]
pub struct PoolModeHandle {
    tx: watch::Sender<bool>,
}

impl PoolModeHandle {
    /// Create a new pool mode handle with initial value
    /// Returns (SharedPoolMode, Receiver) - use receiver in select! loops
    pub fn new(initial: bool) -> (SharedPoolMode, watch::Receiver<bool>) {
        let (tx, rx) = watch::channel(initial);
        (Arc::new(Self { tx }), rx)
    }

    /// Set pool mode (runtime change)
    pub fn set(&self, enabled: bool) {
        let _ = self.tx.send(enabled);
    }

    /// Get current pool mode
    pub fn get(&self) -> bool {
        *self.tx.borrow()
    }
}
