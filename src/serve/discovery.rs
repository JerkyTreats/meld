//! Served-surface discovery: how a second process finds the live listener.
//!
//! The serving process writes one JSON file under the product root with
//! its loopback address and process id; consumers read it instead of
//! guessing ports. The file is observational — a stale file (crash
//! leftover) is detected by the consumer's connection failing, never
//! trusted on its own — and its removal on shutdown is best effort.

use std::fs;
use std::io::Write;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Discovery file name under the product root.
pub const DISCOVERY_FILE: &str = "serve.json";

/// One serving process's advertised surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServeDiscovery {
    /// Loopback address the `/v1` surface listens on.
    pub addr: String,
    /// Operating system process id of the serving process.
    pub process_id: u32,
}

/// Path of the discovery file under one product root.
pub fn discovery_path(product_root: &Path) -> PathBuf {
    product_root.join(DISCOVERY_FILE)
}

/// Advertise one live surface; overwrites a stale advertisement.
pub fn write(product_root: &Path, addr: SocketAddr) -> std::io::Result<()> {
    let record = ServeDiscovery {
        addr: addr.to_string(),
        process_id: std::process::id(),
    };
    let body = serde_json::to_vec_pretty(&record)
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    fs::create_dir_all(product_root)?;
    let mut pending = tempfile::NamedTempFile::new_in(product_root)?;
    pending.write_all(&body)?;
    pending.as_file().sync_all()?;
    pending
        .persist(discovery_path(product_root))
        .map_err(|e| e.error)?;
    Ok(())
}

/// Read the advertised surface, if any process has advertised one.
pub fn read(product_root: &Path) -> Option<ServeDiscovery> {
    let body = fs::read(discovery_path(product_root)).ok()?;
    serde_json::from_slice(&body).ok()
}

/// Withdraw the advertisement; best effort by design.
pub fn remove(product_root: &Path) {
    let _ = fs::remove_file(discovery_path(product_root));
}
