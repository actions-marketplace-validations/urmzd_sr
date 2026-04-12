use crate::ai::services::commit::CommitPlan;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::fingerprint::sha256_hex;

const MAX_ENTRIES: usize = 20;
const TTL_SECS: u64 = 24 * 60 * 60; // 24 hours

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub state_key: String,
    pub fingerprints: BTreeMap<String, String>,
    pub plan: CommitPlan,
    pub created_at: u64,
    pub backend: String,
    pub model: String,
}

/// Return the cache directory for a given repo root:
/// `~/.cache/sr/ai/<repo-id>/entries/`
pub fn cache_dir(repo_root: &Path) -> Option<PathBuf> {
    let base = dirs::cache_dir()?;
    let repo_id = &sha256_hex(repo_root.to_string_lossy().as_bytes())[..16];
    Some(base.join("sr").join("ai").join(repo_id).join("entries"))
}

pub fn entry_path(dir: &Path, state_key: &str) -> PathBuf {
    dir.join(format!("{state_key}.json"))
}

pub fn read_entry(path: &Path) -> Result<CacheEntry> {
    let data = fs::read_to_string(path).context("reading cache entry")?;
    serde_json::from_str(&data).context("parsing cache entry")
}

pub fn write_entry(dir: &Path, entry: &CacheEntry) -> Result<()> {
    fs::create_dir_all(dir).context("creating cache directory")?;
    let path = entry_path(dir, &entry.state_key);
    let data = serde_json::to_string_pretty(entry).context("serializing cache entry")?;
    fs::write(&path, data).context("writing cache entry")?;
    evict(dir)?;
    Ok(())
}

/// List all entries sorted by creation time (newest first).
pub fn list_entries(dir: &Path) -> Result<Vec<CacheEntry>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    for de in fs::read_dir(dir).context("reading cache directory")? {
        let de = de?;
        let path = de.path();
        if path.extension().is_some_and(|e| e == "json")
            && let Ok(entry) = read_entry(&path)
        {
            entries.push(entry);
        }
    }
    entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(entries)
}

/// Evict expired entries and enforce LRU cap.
fn evict(dir: &Path) -> Result<()> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut entries: Vec<(PathBuf, u64)> = Vec::new();

    if let Ok(rd) = fs::read_dir(dir) {
        for de in rd.flatten() {
            let path = de.path();
            if path.extension().is_some_and(|e| e == "json")
                && let Ok(entry) = read_entry(&path)
            {
                if now.saturating_sub(entry.created_at) > TTL_SECS {
                    let _ = fs::remove_file(&path);
                } else {
                    entries.push((path, entry.created_at));
                }
            }
        }
    }

    // LRU: remove oldest entries beyond MAX_ENTRIES
    entries.sort_by(|a, b| b.1.cmp(&a.1));
    for (path, _) in entries.iter().skip(MAX_ENTRIES) {
        let _ = fs::remove_file(path);
    }

    Ok(())
}

/// Clear entries for one repo.
pub fn clear(dir: &Path) -> Result<usize> {
    if !dir.exists() {
        return Ok(0);
    }

    let mut count = 0;
    for de in fs::read_dir(dir)?.flatten() {
        let path = de.path();
        if path.extension().is_some_and(|e| e == "json") {
            let _ = fs::remove_file(&path);
            count += 1;
        }
    }
    Ok(count)
}

/// Clear all repos' caches.
pub fn clear_all() -> Result<usize> {
    let base = dirs::cache_dir()
        .map(|d| d.join("sr").join("ai"))
        .filter(|d| d.exists());

    let Some(base) = base else {
        return Ok(0);
    };

    let mut count = 0;
    for repo_dir in fs::read_dir(&base)?.flatten() {
        let entries_dir = repo_dir.path().join("entries");
        if entries_dir.is_dir() {
            count += clear(&entries_dir)?;
        }
    }

    Ok(count)
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
