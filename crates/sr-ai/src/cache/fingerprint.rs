use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

/// Compute per-file SHA-256 fingerprints for changed files.
///
/// - Tracked files: hash their diff output
/// - Untracked files: hash their full content
///
/// Returns a sorted map of `file_path -> hex_hash`.
pub fn compute_fingerprints(repo_root: &Path, staged_only: bool) -> BTreeMap<String, String> {
    let mut fingerprints = BTreeMap::new();

    if staged_only {
        // Staged tracked files: hash per-file diff
        if let Some(files) = git(repo_root, &["diff", "--cached", "--name-only"]) {
            for file in files.lines().filter(|l| !l.is_empty()) {
                if let Some(diff) = git(repo_root, &["diff", "--cached", "--", file]) {
                    fingerprints.insert(file.to_string(), sha256_hex(diff.as_bytes()));
                }
            }
        }
    } else {
        // All tracked changes (staged + unstaged)
        // Try diff HEAD first; fall back to diff for initial commits
        let diff_files = git(repo_root, &["diff", "HEAD", "--name-only"])
            .or_else(|| git(repo_root, &["diff", "--name-only"]));

        if let Some(files) = diff_files {
            for file in files.lines().filter(|l| !l.is_empty()) {
                let diff = git(repo_root, &["diff", "HEAD", "--", file])
                    .or_else(|| git(repo_root, &["diff", "--", file]));
                if let Some(diff) = diff {
                    fingerprints.insert(file.to_string(), sha256_hex(diff.as_bytes()));
                }
            }
        }

        // Also include staged files not covered by diff HEAD
        if let Some(files) = git(repo_root, &["diff", "--cached", "--name-only"]) {
            for file in files.lines().filter(|l| !l.is_empty()) {
                if !fingerprints.contains_key(file)
                    && let Some(diff) = git(repo_root, &["diff", "--cached", "--", file])
                {
                    fingerprints.insert(file.to_string(), sha256_hex(diff.as_bytes()));
                }
            }
        }

        // Untracked files: hash content
        if let Some(untracked) = git(repo_root, &["ls-files", "--others", "--exclude-standard"]) {
            for file in untracked.lines().filter(|l| !l.is_empty()) {
                let full_path = repo_root.join(file);
                if let Ok(content) = std::fs::read(&full_path) {
                    fingerprints.insert(file.to_string(), sha256_hex(&content));
                }
            }
        }
    }

    fingerprints
}

/// Hash the staged blob content for a file using `git show :0:<path>`.
///
/// This is deterministic (unlike diff-based hashing, which varies by base).
/// Returns `None` if the file is not staged or git fails.
pub fn staged_blob_hash(repo_root: &Path, path: &str) -> Option<String> {
    let spec = format!(":0:{path}");
    let output = Command::new("git")
        .args(["-C", repo_root.to_str()?])
        .args(["show", &spec])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    Some(sha256_hex(&output.stdout))
}

pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

fn git(repo_root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(["-C", repo_root.to_str()?])
        .args(args)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    Some(String::from_utf8_lossy(&output.stdout).to_string())
}
