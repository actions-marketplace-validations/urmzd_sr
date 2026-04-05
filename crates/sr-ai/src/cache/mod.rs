pub mod fingerprint;
pub mod store;

use crate::commands::commit::CommitPlan;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use fingerprint::{compute_fingerprints, sha256_hex};
use store::{CacheEntry, cache_dir, list_entries, read_entry, write_entry};

/// Result of a cache lookup.
pub enum CacheLookup {
    /// Exact fingerprint match — use cached plan directly.
    ExactHit(CommitPlan),
    /// Some files changed — patched plan with affected commits marked.
    /// When `unplaced_files` is empty, the plan can be executed directly
    /// (no AI call needed). When non-empty, AI is called with a targeted
    /// prompt to place only the new files.
    PatchHit {
        plan: CommitPlan,
        /// Indices of commits that contain changed/removed files.
        dirty_commits: Vec<usize>,
        /// Files that changed content (exist in a commit but hash differs).
        changed_files: Vec<String>,
        /// New files not belonging to any cached commit (need AI placement).
        unplaced_files: Vec<String>,
        /// Human-readable delta summary for AI prompt.
        delta_summary: String,
    },
    /// Too much changed or no cache — full AI run.
    Miss,
}

pub struct CacheManager {
    repo_root: PathBuf,
    dir: PathBuf,
    fingerprints: BTreeMap<String, String>,
    state_key: String,
}

impl CacheManager {
    /// Build a new CacheManager, computing fingerprints and state key.
    /// Returns None if cache dir can't be resolved (graceful degradation).
    pub fn new(
        repo_root: &Path,
        staged_only: bool,
        user_message: Option<&str>,
        backend: &str,
        model: &str,
    ) -> Option<Self> {
        let dir = cache_dir(repo_root)?;
        let fingerprints = compute_fingerprints(repo_root, staged_only);

        let state_key = compute_state_key(&fingerprints, staged_only, user_message, backend, model);

        Some(Self {
            repo_root: repo_root.to_path_buf(),
            dir,
            fingerprints,
            state_key,
        })
    }

    /// Look up the cache. Returns ExactHit, PatchHit, or Miss.
    ///
    /// **ExactHit**: all file fingerprints match — use plan as-is.
    /// **PatchHit**: some files changed — identifies dirty commits and unplaced files.
    ///   When `unplaced_files` is empty the plan can execute without an AI call.
    /// **Miss**: too much changed (>50%) or no cache.
    pub fn lookup(&self) -> CacheLookup {
        // Tier 1: exact match
        let exact_path = store::entry_path(&self.dir, &self.state_key);
        if let Ok(entry) = read_entry(&exact_path) {
            return CacheLookup::ExactHit(entry.plan);
        }

        // Tier 2: find best candidate for DAG-aware patching
        let entries = match list_entries(&self.dir) {
            Ok(e) => e,
            Err(_) => return CacheLookup::Miss,
        };

        if entries.is_empty() {
            return CacheLookup::Miss;
        }

        // Pick the most recent entry as the patch candidate
        let candidate = &entries[0];
        let delta = compute_delta(&candidate.fingerprints, &self.fingerprints);

        // Bail if >50% changed — not worth patching
        let total = self.fingerprints.len().max(candidate.fingerprints.len());
        let change_count = delta.changed.len() + delta.added.len() + delta.removed.len();
        if total == 0 || change_count * 2 > total {
            return CacheLookup::Miss;
        }

        // Map changed/removed files to commits in the cached plan (dirty commits).
        let affected_files: std::collections::BTreeSet<&str> = delta
            .changed
            .iter()
            .chain(delta.removed.iter())
            .map(|s| s.as_str())
            .collect();

        let mut dirty_commits = Vec::new();
        for (i, commit) in candidate.plan.commits.iter().enumerate() {
            if commit
                .files
                .iter()
                .any(|f| affected_files.contains(f.as_str()))
            {
                dirty_commits.push(i);
            }
        }

        // Files in `added` that don't belong to any commit are "unplaced".
        let plan_files: std::collections::BTreeSet<&str> = candidate
            .plan
            .commits
            .iter()
            .flat_map(|c| c.files.iter().map(|f| f.as_str()))
            .collect();

        let unplaced_files: Vec<String> = delta
            .added
            .iter()
            .filter(|f| !plan_files.contains(f.as_str()))
            .cloned()
            .collect();

        // Build a patched plan: remove files from commits that were deleted,
        // keep the rest as-is. The dirty_commits list tells the caller which
        // commits need re-validation.
        let mut plan = candidate.plan.clone();
        if !delta.removed.is_empty() {
            let removed_set: std::collections::BTreeSet<&str> =
                delta.removed.iter().map(|s| s.as_str()).collect();
            for commit in &mut plan.commits {
                commit.files.retain(|f| !removed_set.contains(f.as_str()));
            }
            // Drop commits that became empty after removal.
            plan.commits.retain(|c| !c.files.is_empty());
        }

        let summary = format_delta_summary(&delta);

        CacheLookup::PatchHit {
            plan,
            dirty_commits,
            changed_files: delta.changed.clone(),
            unplaced_files,
            delta_summary: summary,
        }
    }

    /// Store a plan in the cache.
    pub fn store(&self, plan: &CommitPlan, backend: &str, model: &str) {
        let entry = CacheEntry {
            state_key: self.state_key.clone(),
            fingerprints: self.fingerprints.clone(),
            plan: plan.clone(),
            created_at: store::now_secs(),
            backend: backend.to_string(),
            model: model.to_string(),
        };

        if let Err(e) = write_entry(&self.dir, &entry) {
            eprintln!("Warning: failed to write cache: {e}");
        }
    }

    /// Clear cache for this repo.
    pub fn clear(&self) -> anyhow::Result<usize> {
        store::clear(&self.dir)
    }

    #[allow(dead_code)]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    #[allow(dead_code)]
    pub fn repo_root(&self) -> &Path {
        &self.repo_root
    }
}

/// Compute the full state key from fingerprints + parameters.
fn compute_state_key(
    fingerprints: &BTreeMap<String, String>,
    staged_only: bool,
    user_message: Option<&str>,
    backend: &str,
    model: &str,
) -> String {
    let mut data = String::new();
    for (file, hash) in fingerprints {
        data.push_str(file);
        data.push(':');
        data.push_str(hash);
        data.push('\n');
    }
    data.push_str(&format!("staged:{staged_only}\n"));
    if let Some(msg) = user_message {
        data.push_str(&format!("message:{msg}\n"));
    }
    data.push_str(&format!("backend:{backend}\n"));
    data.push_str(&format!("model:{model}\n"));

    sha256_hex(data.as_bytes())
}

struct FileDelta {
    unchanged: Vec<String>,
    changed: Vec<String>,
    added: Vec<String>,
    removed: Vec<String>,
}

fn compute_delta(old: &BTreeMap<String, String>, new: &BTreeMap<String, String>) -> FileDelta {
    let mut unchanged = Vec::new();
    let mut changed = Vec::new();
    let mut added = Vec::new();
    let mut removed = Vec::new();

    for (file, new_hash) in new {
        match old.get(file) {
            Some(old_hash) if old_hash == new_hash => unchanged.push(file.clone()),
            Some(_) => changed.push(file.clone()),
            None => added.push(file.clone()),
        }
    }

    for file in old.keys() {
        if !new.contains_key(file) {
            removed.push(file.clone());
        }
    }

    FileDelta {
        unchanged,
        changed,
        added,
        removed,
    }
}

fn format_delta_summary(delta: &FileDelta) -> String {
    let mut parts = Vec::new();

    if !delta.unchanged.is_empty() {
        parts.push(format!(
            "Unchanged files (keep previous groupings): {}",
            delta.unchanged.join(", ")
        ));
    }
    if !delta.changed.is_empty() {
        parts.push(format!(
            "Modified files (re-analyze): {}",
            delta.changed.join(", ")
        ));
    }
    if !delta.added.is_empty() {
        parts.push(format!("New files: {}", delta.added.join(", ")));
    }
    if !delta.removed.is_empty() {
        parts.push(format!(
            "Removed files (drop from plan): {}",
            delta.removed.join(", ")
        ));
    }

    parts.join("\n")
}
