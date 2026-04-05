//! Per-step, per-file hook result cache.
//!
//! Tracks which staged files have already passed each hook step (by content
//! hash), enabling partial retries and skipping unchanged work on big merges.
//!
//! Cache location: `~/.cache/sr/hooks/<repo-id>/step-cache.json`

use crate::error::ReleaseError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const CACHE_VERSION: u32 = 1;
const TTL_SECS: u64 = 7 * 24 * 60 * 60; // 7 days

/// Per-step cache of file content hashes that have passed hook checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepCache {
    pub version: u32,
    pub updated_at: u64,
    /// step_name → (file_path → content_hash)
    pub steps: BTreeMap<String, BTreeMap<String, String>>,
}

impl Default for StepCache {
    fn default() -> Self {
        Self {
            version: CACHE_VERSION,
            updated_at: now_secs(),
            steps: BTreeMap::new(),
        }
    }
}

/// Result of checking which files need re-running for a step.
pub struct StepDiff {
    /// Files whose content hash differs from cache (or not in cache).
    pub changed: Vec<String>,
    /// Files whose content hash matches the cache.
    pub cached: Vec<String>,
}

/// Resolve cache directory: `~/.cache/sr/hooks/<repo-id>/`
pub fn cache_dir(repo_root: &Path) -> Option<PathBuf> {
    let base = dirs::cache_dir()?;
    let repo_id = &sha256_hex(repo_root.to_string_lossy().as_bytes())[..16];
    Some(base.join("sr").join("hooks").join(repo_id))
}

fn cache_path(repo_root: &Path) -> Option<PathBuf> {
    cache_dir(repo_root).map(|d| d.join("step-cache.json"))
}

/// Load the step cache from disk. Returns `Default` on any error (graceful
/// degradation — worst case is a full re-run).
pub fn load_step_cache(repo_root: &Path) -> StepCache {
    let Some(path) = cache_path(repo_root) else {
        return StepCache::default();
    };

    let Ok(data) = std::fs::read_to_string(&path) else {
        return StepCache::default();
    };

    let Ok(cache) = serde_json::from_str::<StepCache>(&data) else {
        return StepCache::default();
    };

    // Version mismatch — start fresh
    if cache.version != CACHE_VERSION {
        return StepCache::default();
    }

    // TTL expired — start fresh
    if now_secs().saturating_sub(cache.updated_at) > TTL_SECS {
        return StepCache::default();
    }

    cache
}

/// Save the step cache to disk.
pub fn save_step_cache(repo_root: &Path, cache: &StepCache) -> Result<(), ReleaseError> {
    let path = cache_path(repo_root)
        .ok_or_else(|| ReleaseError::Config("cannot resolve hook cache directory".into()))?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| ReleaseError::Config(format!("failed to create hook cache dir: {e}")))?;
    }

    let data = serde_json::to_string_pretty(cache)
        .map_err(|e| ReleaseError::Config(format!("failed to serialize hook cache: {e}")))?;

    std::fs::write(&path, data)
        .map_err(|e| ReleaseError::Config(format!("failed to write hook cache: {e}")))?;

    Ok(())
}

/// Hash the staged blob content for a file using `git show :0:<path>`.
///
/// Deterministic — unlike diff-based hashing, the result depends only on the
/// staged content, not the base.
pub fn staged_content_hash(repo_root: &Path, file: &str) -> Option<String> {
    let spec = format!(":0:{file}");
    let output = std::process::Command::new("git")
        .args(["-C", repo_root.to_str()?])
        .args(["show", &spec])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    Some(sha256_hex(&output.stdout))
}

/// Compute content hashes for a set of staged files.
pub fn hash_staged_files(repo_root: &Path, files: &[String]) -> BTreeMap<String, String> {
    let mut result = BTreeMap::new();
    for file in files {
        if let Some(hash) = staged_content_hash(repo_root, file) {
            result.insert(file.clone(), hash);
        }
    }
    result
}

/// Determine which files have changed since they were last cached for a step.
pub fn changed_files_for_step(
    cache: &StepCache,
    step_name: &str,
    current_hashes: &BTreeMap<String, String>,
) -> StepDiff {
    let cached_step = cache.steps.get(step_name);
    let mut changed = Vec::new();
    let mut cached = Vec::new();

    for (file, hash) in current_hashes {
        let is_cached = cached_step
            .and_then(|s| s.get(file))
            .is_some_and(|h| h == hash);

        if is_cached {
            cached.push(file.clone());
        } else {
            changed.push(file.clone());
        }
    }

    StepDiff { changed, cached }
}

/// Record that all files with the given hashes passed a step.
pub fn record_step_pass(cache: &mut StepCache, step_name: &str, hashes: &BTreeMap<String, String>) {
    let step_entry = cache.steps.entry(step_name.to_string()).or_default();
    for (file, hash) in hashes {
        step_entry.insert(file.clone(), hash.clone());
    }
    cache.updated_at = now_secs();
}

fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_cache_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let repo_root = dir.path();

        let mut cache = StepCache::default();
        let mut hashes = BTreeMap::new();
        hashes.insert("src/main.rs".to_string(), "abc123".to_string());
        hashes.insert("src/lib.rs".to_string(), "def456".to_string());
        record_step_pass(&mut cache, "format", &hashes);

        save_step_cache(repo_root, &cache).unwrap();
        let loaded = load_step_cache(repo_root);

        assert_eq!(loaded.version, CACHE_VERSION);
        assert_eq!(
            loaded
                .steps
                .get("format")
                .unwrap()
                .get("src/main.rs")
                .unwrap(),
            "abc123"
        );
    }

    #[test]
    fn changed_files_detection() {
        let mut cache = StepCache::default();
        let mut old_hashes = BTreeMap::new();
        old_hashes.insert("a.rs".to_string(), "hash_a".to_string());
        old_hashes.insert("b.rs".to_string(), "hash_b".to_string());
        record_step_pass(&mut cache, "lint", &old_hashes);

        // b.rs changed, c.rs is new
        let mut current = BTreeMap::new();
        current.insert("a.rs".to_string(), "hash_a".to_string());
        current.insert("b.rs".to_string(), "hash_b_new".to_string());
        current.insert("c.rs".to_string(), "hash_c".to_string());

        let diff = changed_files_for_step(&cache, "lint", &current);
        assert_eq!(diff.cached, vec!["a.rs"]);
        assert_eq!(diff.changed, vec!["b.rs", "c.rs"]);
    }

    #[test]
    fn empty_cache_all_changed() {
        let cache = StepCache::default();
        let mut current = BTreeMap::new();
        current.insert("a.rs".to_string(), "hash_a".to_string());

        let diff = changed_files_for_step(&cache, "lint", &current);
        assert!(diff.cached.is_empty());
        assert_eq!(diff.changed, vec!["a.rs"]);
    }

    #[test]
    fn expired_cache_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let repo_root = dir.path();

        let mut cache = StepCache::default();
        cache.updated_at = 0; // epoch — definitely expired
        save_step_cache(repo_root, &cache).unwrap();

        let loaded = load_step_cache(repo_root);
        assert!(loaded.steps.is_empty());
    }
}
