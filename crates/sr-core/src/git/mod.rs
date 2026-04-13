use anyhow::{Context, Result, bail};
use semver::Version;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

use crate::commit::Commit;
use crate::error::ReleaseError;

fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Information about a git tag.
#[derive(Debug, Clone)]
pub struct TagInfo {
    pub name: String,
    pub version: Version,
    pub sha: String,
}

/// Abstraction over git operations.
pub trait GitRepository: Send + Sync {
    /// Find the latest semver tag matching the configured prefix.
    fn latest_tag(&self, prefix: &str) -> Result<Option<TagInfo>, ReleaseError>;

    /// List commits between a starting point (exclusive) and HEAD (inclusive).
    /// If `from` is `None`, returns all commits reachable from HEAD.
    fn commits_since(&self, from: Option<&str>) -> Result<Vec<Commit>, ReleaseError>;

    /// Create an annotated tag at HEAD. When `sign` is true, uses `-s` for GPG/SSH signing.
    fn create_tag(&self, name: &str, message: &str, sign: bool) -> Result<(), ReleaseError>;

    /// Push a tag to the remote.
    fn push_tag(&self, name: &str) -> Result<(), ReleaseError>;

    /// Stage files and commit (skips git hooks via --no-verify).
    /// Returns Ok(false) if nothing to commit.
    fn stage_and_commit(&self, paths: &[&str], message: &str) -> Result<bool, ReleaseError>;

    /// Check if the working tree has uncommitted changes.
    fn is_dirty(&self) -> Result<bool, ReleaseError>;

    /// Push current branch to origin.
    fn push(&self) -> Result<(), ReleaseError>;

    /// Check if a tag exists locally.
    fn tag_exists(&self, name: &str) -> Result<bool, ReleaseError>;

    /// Check if a tag exists on the remote.
    fn remote_tag_exists(&self, name: &str) -> Result<bool, ReleaseError>;

    /// List all semver tags matching prefix, sorted by version ascending.
    fn all_tags(&self, prefix: &str) -> Result<Vec<TagInfo>, ReleaseError>;

    /// List commits between two refs (exclusive `from`, inclusive `to`).
    /// If `from` is None, returns all commits reachable from `to`.
    fn commits_between(&self, from: Option<&str>, to: &str) -> Result<Vec<Commit>, ReleaseError>;

    /// Get the date (YYYY-MM-DD) of the commit a tag points to.
    fn tag_date(&self, tag_name: &str) -> Result<String, ReleaseError>;

    /// Force-create a lightweight tag at HEAD, overwriting if it already exists.
    fn force_create_tag(&self, name: &str) -> Result<(), ReleaseError>;

    /// Force-push a tag to the remote, overwriting the remote tag if it exists.
    fn force_push_tag(&self, name: &str) -> Result<(), ReleaseError>;

    /// Return the full SHA of HEAD.
    fn head_sha(&self) -> Result<String, ReleaseError>;

    /// Like `commits_since`, but only includes commits that touched files under `path`.
    fn commits_since_in_path(
        &self,
        from: Option<&str>,
        path: &str,
    ) -> Result<Vec<Commit>, ReleaseError> {
        // Default: ignore path filter (for test fakes and backwards compat)
        let _ = path;
        self.commits_since(from)
    }

    /// Like `commits_between`, but only includes commits that touched files under `path`.
    fn commits_between_in_path(
        &self,
        from: Option<&str>,
        to: &str,
        path: &str,
    ) -> Result<Vec<Commit>, ReleaseError> {
        let _ = path;
        self.commits_between(from, to)
    }
}

/// Strip C-style quoting that git applies to paths containing spaces,
/// non-ASCII characters, or other special bytes. Git wraps such paths
/// in double quotes and uses backslash escapes (e.g. `\t`, `\n`, `\\`,
/// `\"`, and octal `\NNN`).
fn git_unquote(s: &str) -> String {
    let s = s.trim();
    if !(s.starts_with('"') && s.ends_with('"')) {
        return s.to_string();
    }
    // Strip surrounding quotes
    let inner = &s[1..s.len() - 1];
    let mut out = Vec::new();
    let bytes = inner.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            i += 1;
            match bytes[i] {
                b'\\' => out.push(b'\\'),
                b'"' => out.push(b'"'),
                b'n' => out.push(b'\n'),
                b't' => out.push(b'\t'),
                b'r' => out.push(b'\r'),
                b'a' => out.push(0x07),
                b'b' => out.push(0x08),
                b'f' => out.push(0x0C),
                b'v' => out.push(0x0B),
                // Octal escape: \NNN (1-3 digits)
                b'0'..=b'3' => {
                    let mut val = (bytes[i] - b'0') as u16;
                    for _ in 0..2 {
                        if i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit() {
                            i += 1;
                            val = val * 8 + (bytes[i] - b'0') as u16;
                        } else {
                            break;
                        }
                    }
                    out.push(val as u8);
                }
                other => {
                    out.push(b'\\');
                    out.push(other);
                }
            }
        } else {
            out.push(bytes[i]);
        }
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).to_string())
}

pub struct GitRepo {
    root: PathBuf,
}

#[allow(dead_code)]
impl GitRepo {
    pub fn discover() -> Result<Self> {
        let output = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .output()
            .context("failed to run git")?;

        if !output.status.success() {
            bail!("not in a git repository");
        }

        let root = String::from_utf8(output.stdout)
            .context("invalid utf-8 from git")?
            .trim()
            .into();

        Ok(Self { root })
    }

    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    fn git(&self, args: &[&str]) -> Result<String> {
        let output = Command::new("git")
            .args(["-C", self.root.to_str().unwrap()])
            .args(args)
            .output()
            .with_context(|| format!("failed to run git {}", args.join(" ")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("git {} failed: {}", args.join(" "), stderr.trim());
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn git_allow_failure(&self, args: &[&str]) -> Result<(bool, String)> {
        let output = Command::new("git")
            .args(["-C", self.root.to_str().unwrap()])
            .args(args)
            .output()
            .with_context(|| format!("failed to run git {}", args.join(" ")))?;

        Ok((
            output.status.success(),
            String::from_utf8_lossy(&output.stdout).to_string(),
        ))
    }

    pub fn has_staged_changes(&self) -> Result<bool> {
        let out = self.git(&["diff", "--cached", "--name-only"])?;
        Ok(!out.trim().is_empty())
    }

    pub fn has_any_changes(&self) -> Result<bool> {
        let out = self.git(&["status", "--porcelain"])?;
        Ok(!out.trim().is_empty())
    }

    pub fn has_head(&self) -> Result<bool> {
        let (ok, _) = self.git_allow_failure(&["rev-parse", "HEAD"])?;
        Ok(ok)
    }

    pub fn reset_head(&self) -> Result<()> {
        if self.has_head()? {
            self.git(&["reset", "HEAD", "--quiet"])?;
        } else {
            // Fresh repo with no commits — unstage via rm --cached
            let _ = self.git_allow_failure(&["rm", "--cached", "-r", ".", "--quiet"]);
        }
        Ok(())
    }

    pub fn stage_file(&self, file: &str) -> Result<bool> {
        // Let git decide whether the file can be staged. This handles:
        //   - existing files (additions/modifications)
        //   - tracked files deleted from the working tree (deletions/moves)
        //   - files that don't exist and aren't tracked (returns false)
        // Previous code ran `git ls-files --deleted` per file as a pre-check,
        // which was O(n²) for many deletes and could fail when path formats
        // differed between git commands (e.g. C-quoted vs unquoted paths).
        let (ok, _) = self.git_allow_failure(&["add", "--", file])?;
        Ok(ok)
    }

    pub fn has_staged_after_add(&self) -> Result<bool> {
        self.has_staged_changes()
    }

    pub fn commit(&self, message: &str) -> Result<()> {
        let output = Command::new("git")
            .args(["-C", self.root.to_str().unwrap()])
            .args(["commit", "-F", "-"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("failed to spawn git commit")?;

        use std::io::Write;
        let mut child = output;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(message.as_bytes())?;
        }

        let out = child.wait_with_output()?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            bail!("git commit failed: {}", stderr.trim());
        }

        Ok(())
    }

    pub fn recent_commits(&self, count: usize) -> Result<String> {
        self.git(&["--no-pager", "log", "--oneline", &format!("-{count}")])
    }

    pub fn diff_cached(&self) -> Result<String> {
        self.git(&["diff", "--cached"])
    }

    pub fn diff_cached_stat(&self) -> Result<String> {
        self.git(&["diff", "--cached", "--stat"])
    }

    pub fn diff_head(&self) -> Result<String> {
        let (ok, out) = self.git_allow_failure(&["diff", "HEAD"])?;
        if ok { Ok(out) } else { self.git(&["diff"]) }
    }

    /// Run `git diff` with explicit context lines and optional file filter.
    /// When `staged` is true, uses `--cached`; otherwise diffs against HEAD.
    pub fn diff_unified(&self, staged: bool, context: usize, files: &[String]) -> Result<String> {
        let ctx_flag = format!("-U{context}");
        let mut args: Vec<&str> = vec!["diff", &ctx_flag];
        if staged {
            args.push("--cached");
        } else {
            args.push("HEAD");
        }
        if !files.is_empty() {
            args.push("--");
            for f in files {
                args.push(f.as_str());
            }
        }
        let (ok, out) = self.git_allow_failure(&args)?;
        if ok {
            Ok(out)
        } else if !staged && files.is_empty() {
            // No HEAD yet — fall back to plain diff
            self.git(&["diff", &ctx_flag])
        } else {
            Ok(out)
        }
    }

    /// Return per-file numstat: `(additions, deletions, path)`.
    pub fn diff_numstat(&self, staged: bool, files: &[String]) -> Result<Vec<(usize, usize, String)>> {
        let mut args: Vec<&str> = vec!["diff", "--numstat"];
        if staged {
            args.push("--cached");
        } else {
            args.push("HEAD");
        }
        if !files.is_empty() {
            args.push("--");
            for f in files {
                args.push(f.as_str());
            }
        }
        let (ok, out) = self.git_allow_failure(&args)?;
        if !ok && !staged && files.is_empty() {
            let out = self.git(&["diff", "--numstat"])?;
            return Self::parse_numstat(&out);
        }
        Self::parse_numstat(&out)
    }

    fn parse_numstat(out: &str) -> Result<Vec<(usize, usize, String)>> {
        let mut result = Vec::new();
        for line in out.lines() {
            let parts: Vec<&str> = line.splitn(3, '\t').collect();
            if parts.len() == 3 {
                // Binary files show "-" for additions/deletions
                let add = parts[0].parse().unwrap_or(0);
                let del = parts[1].parse().unwrap_or(0);
                let path = if let Some(pos) = parts[2].find(" => ") {
                    // Rename: "old => new" or "{old => new}/path"
                    git_unquote(&parts[2][pos + 4..])
                } else {
                    git_unquote(parts[2])
                };
                result.push((add, del, path));
            }
        }
        Ok(result)
    }

    pub fn status_porcelain(&self) -> Result<String> {
        self.git(&["status", "--porcelain"])
    }

    pub fn untracked_files(&self) -> Result<String> {
        self.git(&["ls-files", "--others", "--exclude-standard"])
    }

    pub fn show(&self, rev: &str) -> Result<String> {
        self.git(&["show", rev])
    }

    pub fn log_range(&self, base: &str, count: Option<usize>) -> Result<String> {
        let mut args = vec!["--no-pager", "log", "--oneline"];
        let count_str;
        if let Some(n) = count {
            count_str = format!("-{n}");
            args.push(&count_str);
        }
        args.push(base);
        self.git(&args)
    }

    pub fn diff_range(&self, base: &str) -> Result<String> {
        self.git(&["diff", base])
    }

    pub fn current_branch(&self) -> Result<String> {
        let out = self.git(&["rev-parse", "--abbrev-ref", "HEAD"])?;
        Ok(out.trim().to_string())
    }

    pub fn head_short(&self) -> Result<String> {
        let out = self.git(&["rev-parse", "--short", "HEAD"])?;
        Ok(out.trim().to_string())
    }

    /// Count commits since the last tag. If no tags exist, counts all commits.
    pub fn commits_since_last_tag(&self) -> Result<usize> {
        // Try to find the most recent tag
        let (ok, tag) = self.git_allow_failure(&["describe", "--tags", "--abbrev=0"])?;
        let tag = tag.trim();

        let out = if ok && !tag.is_empty() {
            self.git(&["rev-list", &format!("{tag}..HEAD"), "--count"])?
        } else {
            self.git(&["rev-list", "HEAD", "--count"])?
        };

        out.trim()
            .parse::<usize>()
            .context("failed to parse commit count")
    }

    /// Get detailed log of recent commits (SHA, subject, body) oldest first.
    pub fn log_detailed(&self, count: usize) -> Result<String> {
        let out = self.git(&[
            "--no-pager",
            "log",
            "--reverse",
            &format!("-{count}"),
            "--format=%h %s%n%b%n---",
        ])?;
        Ok(out)
    }

    pub fn file_statuses(&self) -> Result<HashMap<String, char>> {
        let out = self.git(&["status", "--porcelain"])?;
        let mut map = HashMap::new();
        for line in out.lines() {
            if line.len() < 3 {
                continue;
            }
            let xy = &line.as_bytes()[..2];
            let path = line[3..].to_string();
            let (x, y) = (xy[0], xy[1]);
            let is_rename = matches!((x, y), (b'R', _) | (_, b'R'));
            if is_rename {
                if let Some(pos) = path.find(" -> ") {
                    let old_path = git_unquote(&path[..pos]);
                    let new_path = git_unquote(&path[pos + 4..]);
                    map.insert(old_path, 'D');
                    map.insert(new_path, 'R');
                } else {
                    map.insert(git_unquote(&path), 'R');
                }
            } else {
                let status = match (x, y) {
                    (b'?', b'?') => 'A',
                    (b'A', _) | (_, b'A') => 'A',
                    (b'D', _) | (_, b'D') => 'D',
                    (b'M', _) | (_, b'M') | (b'T', _) | (_, b'T') => 'M',
                    _ => '~',
                };
                map.insert(git_unquote(&path), status);
            }
        }
        Ok(map)
    }

    /// Create a snapshot of the working tree state into the platform data directory.
    /// Location: `<data_local_dir>/sr/snapshots/<repo-hash>/`
    ///   - macOS:   ~/Library/Application Support/sr/snapshots/<hash>/
    ///   - Linux:   ~/.local/share/sr/snapshots/<hash>/
    ///   - Windows: %LOCALAPPDATA%/sr/snapshots/<hash>/
    ///
    /// The snapshot directly copies every changed/added/deleted file into
    /// `files/` alongside a `manifest.json` that records each file's status
    /// and whether it was staged. This avoids git-stash entirely — restore
    /// is a plain file copy that cannot conflict.
    ///
    /// Lives completely outside the repo so the agent cannot touch it.
    pub fn snapshot_working_tree(&self) -> Result<PathBuf> {
        let snapshot_dir = snapshot_dir_for(&self.root)
            .context("failed to resolve snapshot directory (no data directory available)")?;
        // Start fresh — remove any prior snapshot for this repo
        if snapshot_dir.exists() {
            std::fs::remove_dir_all(&snapshot_dir).ok();
        }
        std::fs::create_dir_all(&snapshot_dir).context("failed to create snapshot directory")?;

        let files_dir = snapshot_dir.join("files");
        std::fs::create_dir_all(&files_dir)?;

        // Record which repo this snapshot belongs to
        std::fs::write(
            snapshot_dir.join("repo_root"),
            self.root.to_string_lossy().as_bytes(),
        )
        .context("failed to write repo_root")?;

        // Record current HEAD so we can reset if partial commits were made
        let (has_head, head_ref) = self.git_allow_failure(&["rev-parse", "HEAD"])?;
        if has_head {
            std::fs::write(snapshot_dir.join("head_ref"), head_ref.trim())
                .context("failed to write head_ref")?;
        }

        // Build manifest: every file that shows up in `git status --porcelain`
        // gets its content copied and its status recorded.
        let porcelain = self.git(&["status", "--porcelain"])?;
        let staged_names = self.git(&["diff", "--cached", "--name-only", "-z"])?;
        let staged_set: std::collections::HashSet<String> = staged_names
            .split('\0')
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();

        #[derive(serde::Serialize, serde::Deserialize)]
        struct ManifestEntry {
            path: String,
            /// X (index) status character from porcelain
            index_status: char,
            /// Y (worktree) status character from porcelain
            worktree_status: char,
            /// Whether the file was staged at snapshot time
            staged: bool,
            /// Whether a file copy exists in the snapshot (false for deletions)
            has_content: bool,
        }

        let mut manifest: Vec<ManifestEntry> = Vec::new();

        for line in porcelain.lines() {
            if line.len() < 3 {
                continue;
            }
            let bytes = line.as_bytes();
            let x = bytes[0] as char;
            let y = bytes[1] as char;
            let raw = line[3..].to_string();
            // Handle renames: "R  old -> new" — keep only the new path
            let path = if let Some(pos) = raw.find(" -> ") {
                git_unquote(&raw[pos + 4..])
            } else {
                git_unquote(&raw)
            };

            let src = self.root.join(&path);
            let has_content = src.exists() && src.is_file();

            if has_content {
                let dest = files_dir.join(&path);
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent).ok();
                }
                if let Err(e) = std::fs::copy(&src, &dest) {
                    eprintln!("warning: failed to snapshot {path}: {e}");
                }
            }

            manifest.push(ManifestEntry {
                staged: staged_set.contains(path.as_str()),
                path,
                index_status: x,
                worktree_status: y,
                has_content,
            });
        }

        let manifest_json =
            serde_json::to_string_pretty(&manifest).context("failed to serialize manifest")?;
        std::fs::write(snapshot_dir.join("manifest.json"), manifest_json)
            .context("failed to write manifest.json")?;

        // Mark snapshot as valid
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        std::fs::write(snapshot_dir.join("timestamp"), now.to_string())
            .context("failed to write timestamp")?;

        Ok(snapshot_dir)
    }

    /// Restore working tree from the latest snapshot.
    ///
    /// 1. Reset HEAD to the original commit (undoes any partial commits)
    /// 2. Clean the index
    /// 3. Copy every snapshotted file back from `files/`
    /// 4. Delete files that were deleted at snapshot time
    /// 5. Re-stage files that were staged at snapshot time
    ///
    /// This is a plain file copy — no git-stash, no merge conflicts.
    pub fn restore_snapshot(&self) -> Result<()> {
        let snapshot_dir = self.snapshot_dir()?;
        if !snapshot_dir.join("timestamp").exists() {
            bail!("no valid snapshot found");
        }

        let files_dir = snapshot_dir.join("files");

        // Step 1: Reset HEAD to pre-operation state
        let head_ref_path = snapshot_dir.join("head_ref");
        if head_ref_path.exists() {
            let original_head = std::fs::read_to_string(&head_ref_path)?;
            let original_head = original_head.trim();
            if !original_head.is_empty() {
                let _ = self.git_allow_failure(&["reset", "--soft", original_head]);
            }
        }

        // Step 2: Clean the index
        self.reset_head()?;

        // Step 3-5: Restore files from manifest
        let manifest_path = snapshot_dir.join("manifest.json");
        if !manifest_path.exists() {
            bail!("snapshot manifest.json missing — cannot restore");
        }

        #[derive(serde::Deserialize)]
        struct ManifestEntry {
            path: String,
            index_status: char,
            worktree_status: char,
            staged: bool,
            has_content: bool,
        }

        let manifest_data = std::fs::read_to_string(&manifest_path)?;
        let manifest: Vec<ManifestEntry> =
            serde_json::from_str(&manifest_data).context("failed to parse snapshot manifest")?;

        let mut restored = 0usize;
        let mut failed = 0usize;

        for entry in &manifest {
            let dest = self.root.join(&entry.path);

            if entry.has_content {
                // Restore file content from snapshot copy
                let src = files_dir.join(&entry.path);
                if src.exists() {
                    if let Some(parent) = dest.parent() {
                        std::fs::create_dir_all(parent).ok();
                    }
                    match std::fs::copy(&src, &dest) {
                        Ok(_) => restored += 1,
                        Err(e) => {
                            eprintln!("warning: failed to restore {}: {e}", entry.path);
                            failed += 1;
                        }
                    }
                } else {
                    eprintln!("warning: snapshot missing content for {}", entry.path);
                    failed += 1;
                }
            } else if entry.index_status == 'D' || entry.worktree_status == 'D' {
                // File was deleted at snapshot time — ensure it stays deleted
                if dest.exists() {
                    std::fs::remove_file(&dest).ok();
                }
            }

            // Re-stage if it was staged at snapshot time
            if entry.staged {
                let _ = self.git_allow_failure(&["add", "--", &entry.path]);
            }
        }

        if failed > 0 {
            eprintln!("sr: restored {restored} files, {failed} failed");
        }

        Ok(())
    }

    /// Remove the snapshot after a successful operation.
    pub fn clear_snapshot(&self) {
        if let Ok(dir) = self.snapshot_dir() {
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    /// Returns the snapshot directory path for this repo.
    pub fn snapshot_dir(&self) -> Result<PathBuf> {
        snapshot_dir_for(&self.root)
            .context("failed to resolve snapshot directory (no data directory available)")
    }

    /// Check if a valid snapshot exists.
    pub fn has_snapshot(&self) -> bool {
        self.snapshot_dir()
            .map(|d| d.join("timestamp").exists())
            .unwrap_or(false)
    }
}

/// Resolve the snapshot directory for a repo root.
/// `<data_local_dir>/sr/snapshots/<repo-hash>/`
fn snapshot_dir_for(repo_root: &std::path::Path) -> Option<PathBuf> {
    let base = dirs::data_local_dir()?;
    let repo_id = &sha256_hex(repo_root.to_string_lossy().as_bytes())[..16];
    Some(base.join("sr").join("snapshots").join(repo_id))
}

/// Guard that ensures the snapshot is cleaned up on success
/// and restored on failure (drop without explicit success).
pub struct SnapshotGuard<'a> {
    repo: &'a GitRepo,
    succeeded: bool,
}

impl<'a> SnapshotGuard<'a> {
    /// Create a snapshot and return the guard.
    pub fn new(repo: &'a GitRepo) -> Result<Self> {
        repo.snapshot_working_tree()?;
        Ok(Self {
            repo,
            succeeded: false,
        })
    }

    /// Mark the operation as successful — snapshot will be cleared on drop.
    pub fn success(mut self) {
        self.succeeded = true;
        self.repo.clear_snapshot();
    }
}

impl Drop for SnapshotGuard<'_> {
    fn drop(&mut self) {
        if !self.succeeded && self.repo.has_snapshot() {
            eprintln!("sr: operation failed, restoring working tree from snapshot...");
            if let Err(e) = self.repo.restore_snapshot() {
                eprintln!("sr: warning: snapshot restore failed: {e}");
                if let Ok(dir) = self.repo.snapshot_dir() {
                    eprintln!(
                        "sr: snapshot preserved at {} for manual recovery",
                        dir.display()
                    );
                }
            } else {
                self.repo.clear_snapshot();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Create a temporary git repo with an initial commit and return a GitRepo.
    fn temp_repo() -> (tempfile::TempDir, GitRepo) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();

        let git = |args: &[&str]| {
            Command::new("git")
                .args(["-C", root.to_str().unwrap()])
                .args(args)
                .output()
                .unwrap()
        };

        git(&["init"]);
        git(&["config", "user.email", "test@test.com"]);
        git(&["config", "user.name", "Test"]);
        // Initial commit so HEAD exists
        fs::write(root.join("init.txt"), "init").unwrap();
        git(&["add", "init.txt"]);
        git(&["commit", "-m", "initial"]);

        let repo = GitRepo { root };
        (dir, repo)
    }

    #[test]
    fn snapshot_creates_manifest_with_staged_files() {
        let (_dir, repo) = temp_repo();

        // Create and stage a new file
        fs::write(repo.root.join("new.go"), "package main").unwrap();
        repo.git(&["add", "new.go"]).unwrap();

        let snap_dir = repo.snapshot_working_tree().unwrap();

        // Manifest should exist
        let manifest_path = snap_dir.join("manifest.json");
        assert!(manifest_path.exists(), "manifest.json should exist");

        let data = fs::read_to_string(&manifest_path).unwrap();
        assert!(data.contains("new.go"), "manifest should list new.go");
        assert!(
            data.contains("\"staged\": true"),
            "new.go should be marked staged"
        );

        // File copy should exist
        assert!(
            snap_dir.join("files/new.go").exists(),
            "file content should be copied"
        );
        assert_eq!(
            fs::read_to_string(snap_dir.join("files/new.go")).unwrap(),
            "package main"
        );

        // HEAD ref should be recorded
        assert!(snap_dir.join("head_ref").exists());

        repo.clear_snapshot();
    }

    #[test]
    fn snapshot_restore_recovers_staged_new_files() {
        let (_dir, repo) = temp_repo();

        // Stage two new files
        fs::write(repo.root.join("a.go"), "package a").unwrap();
        fs::write(repo.root.join("b.go"), "package b").unwrap();
        repo.git(&["add", "a.go", "b.go"]).unwrap();

        repo.snapshot_working_tree().unwrap();

        // Simulate what execute_plan does: reset head, stage partially, commit
        repo.reset_head().unwrap();
        repo.git(&["add", "a.go"]).unwrap();
        repo.git(&["commit", "-m", "partial"]).unwrap();

        // Now restore — should undo the partial commit and recover both files staged
        repo.restore_snapshot().unwrap();

        // Both files should exist
        assert!(repo.root.join("a.go").exists());
        assert!(repo.root.join("b.go").exists());
        assert_eq!(
            fs::read_to_string(repo.root.join("a.go")).unwrap(),
            "package a"
        );
        assert_eq!(
            fs::read_to_string(repo.root.join("b.go")).unwrap(),
            "package b"
        );

        // Both should be staged
        let staged = repo.git(&["diff", "--cached", "--name-only"]).unwrap();
        assert!(staged.contains("a.go"), "a.go should be re-staged");
        assert!(staged.contains("b.go"), "b.go should be re-staged");

        // The partial commit should be gone
        let log = repo.git(&["log", "--oneline"]).unwrap();
        assert!(
            !log.contains("partial"),
            "partial commit should be undone by HEAD reset"
        );

        repo.clear_snapshot();
    }

    #[test]
    fn snapshot_restore_with_dirty_index_does_not_conflict() {
        let (_dir, repo) = temp_repo();

        // Stage a new file
        fs::write(repo.root.join("file.rs"), "fn main() {}").unwrap();
        repo.git(&["add", "file.rs"]).unwrap();

        repo.snapshot_working_tree().unwrap();

        // Simulate partial staging left by a failed execute_plan
        repo.reset_head().unwrap();
        repo.git(&["add", "file.rs"]).unwrap();
        // Don't commit — index is dirty with the same file

        // Restore should NOT fail (this was the original bug)
        let result = repo.restore_snapshot();
        assert!(
            result.is_ok(),
            "restore should succeed with dirty index: {result:?}"
        );

        assert_eq!(
            fs::read_to_string(repo.root.join("file.rs")).unwrap(),
            "fn main() {}"
        );

        repo.clear_snapshot();
    }

    #[test]
    fn snapshot_handles_modified_files() {
        let (_dir, repo) = temp_repo();

        // Modify an existing tracked file
        fs::write(repo.root.join("init.txt"), "modified content").unwrap();
        repo.git(&["add", "init.txt"]).unwrap();

        repo.snapshot_working_tree().unwrap();

        // Simulate: reset and make a different change
        repo.reset_head().unwrap();
        fs::write(repo.root.join("init.txt"), "wrong content").unwrap();

        // Restore should bring back the original modified content
        repo.restore_snapshot().unwrap();

        assert_eq!(
            fs::read_to_string(repo.root.join("init.txt")).unwrap(),
            "modified content"
        );

        repo.clear_snapshot();
    }

    #[test]
    fn snapshot_guard_restores_on_drop() {
        let (_dir, repo) = temp_repo();

        fs::write(repo.root.join("guarded.txt"), "important").unwrap();
        repo.git(&["add", "guarded.txt"]).unwrap();

        {
            let _guard = SnapshotGuard::new(&repo).unwrap();
            // Simulate failure: reset and delete the file
            repo.reset_head().unwrap();
            fs::remove_file(repo.root.join("guarded.txt")).ok();
            // Guard drops here without calling success()
        }

        // File should be restored
        assert!(repo.root.join("guarded.txt").exists());
        assert_eq!(
            fs::read_to_string(repo.root.join("guarded.txt")).unwrap(),
            "important"
        );
    }

    #[test]
    fn snapshot_guard_clears_on_success() {
        let (_dir, repo) = temp_repo();

        fs::write(repo.root.join("ok.txt"), "data").unwrap();
        repo.git(&["add", "ok.txt"]).unwrap();

        let guard = SnapshotGuard::new(&repo).unwrap();
        assert!(repo.has_snapshot());
        guard.success();

        // Snapshot should be cleared
        assert!(!repo.has_snapshot());
    }

    #[test]
    fn file_statuses_includes_both_sides_of_rename() {
        let (_dir, repo) = temp_repo();

        // Create and commit a file
        fs::write(repo.root.join("old_name.txt"), "content").unwrap();
        repo.git(&["add", "old_name.txt"]).unwrap();
        repo.git(&["commit", "-m", "add old_name"]).unwrap();

        // Rename it via git mv
        repo.git(&["mv", "old_name.txt", "new_name.txt"]).unwrap();

        let statuses = repo.file_statuses().unwrap();

        assert_eq!(
            statuses.get("old_name.txt").copied(),
            Some('D'),
            "old path should appear as deleted"
        );
        assert_eq!(
            statuses.get("new_name.txt").copied(),
            Some('R'),
            "new path should appear as renamed"
        );
    }

    /// Simulate the execute_plan flow: many files with moves, deletes, and
    /// modifications. After reset_head(), every path from file_statuses()
    /// must be stageable via stage_file(). This is the scenario that breaks
    /// when there are 100+ changes with moves.
    #[test]
    fn stage_file_handles_many_moves_and_deletes_after_reset() {
        let (_dir, repo) = temp_repo();

        // Create 30 files and commit them
        for i in 0..30 {
            fs::write(
                repo.root.join(format!("file_{i}.txt")),
                format!("content {i}"),
            )
            .unwrap();
        }
        repo.git(&["add", "."]).unwrap();
        repo.git(&["commit", "-m", "add files"]).unwrap();

        // Move files 0..10 into a subdirectory (simulates directory rename)
        fs::create_dir_all(repo.root.join("moved")).unwrap();
        for i in 0..10 {
            repo.git(&[
                "mv",
                &format!("file_{i}.txt"),
                &format!("moved/file_{i}.txt"),
            ])
            .unwrap();
        }

        // Delete files 10..20
        for i in 10..20 {
            repo.git(&["rm", &format!("file_{i}.txt")]).unwrap();
        }

        // Modify files 20..30
        for i in 20..30 {
            fs::write(
                repo.root.join(format!("file_{i}.txt")),
                format!("modified {i}"),
            )
            .unwrap();
            repo.git(&["add", &format!("file_{i}.txt")]).unwrap();
        }

        // Add some new files too
        for i in 30..35 {
            fs::write(repo.root.join(format!("new_{i}.txt")), format!("new {i}")).unwrap();
            repo.git(&["add", &format!("new_{i}.txt")]).unwrap();
        }

        // Capture statuses before reset (this is what the AI sees)
        let statuses = repo.file_statuses().unwrap();
        assert!(
            statuses.len() >= 30,
            "should have many file statuses, got {}",
            statuses.len()
        );

        // Reset head — exactly what execute_plan does
        repo.reset_head().unwrap();

        // Now try to stage every file from statuses — this is what execute_plan does
        let mut failed = Vec::new();
        for (file, status) in &statuses {
            if file == "init.txt" {
                continue;
            }
            let ok = repo.stage_file(file).unwrap();
            if !ok {
                failed.push((file.clone(), *status));
            }
        }

        assert!(
            failed.is_empty(),
            "stage_file failed for {} files: {:?}",
            failed.len(),
            failed
        );
    }

    /// Test that stage_file works when files are moved MANUALLY (not git mv)
    /// and then staged with git add. This is the common case for directory
    /// renames where users just mv the directory and git add everything.
    #[test]
    fn stage_file_handles_manual_moves_after_reset() {
        let (_dir, repo) = temp_repo();

        // Create files in a directory and commit
        fs::create_dir_all(repo.root.join("old_dir")).unwrap();
        for i in 0..10 {
            fs::write(
                repo.root.join(format!("old_dir/file_{i}.txt")),
                format!("content {i}"),
            )
            .unwrap();
        }
        repo.git(&["add", "."]).unwrap();
        repo.git(&["commit", "-m", "add directory"]).unwrap();

        // Manually move the directory (simulates user doing: mv old_dir new_dir)
        fs::rename(repo.root.join("old_dir"), repo.root.join("new_dir")).unwrap();

        // Stage everything (simulates: git add -A)
        repo.git(&["add", "-A"]).unwrap();

        // Capture statuses
        let statuses = repo.file_statuses().unwrap();

        // Reset head — like execute_plan does
        repo.reset_head().unwrap();

        // Try to stage every file
        let mut failed = Vec::new();
        for (file, status) in &statuses {
            if file == "init.txt" {
                continue;
            }
            let ok = repo.stage_file(file).unwrap();
            if !ok {
                failed.push((file.clone(), *status));
            }
        }

        assert!(
            failed.is_empty(),
            "stage_file failed for {} files after manual move: {:?}",
            failed.len(),
            failed
        );
    }

    /// Test that stage_file works when new (uncommitted) files are involved
    /// alongside moves and deletes. New files that were staged but never
    /// committed are tricky because after reset_head() they drop out of
    /// the index entirely.
    #[test]
    fn stage_file_handles_new_files_mixed_with_moves() {
        let (_dir, repo) = temp_repo();

        // Create and commit existing files
        for i in 0..5 {
            fs::write(
                repo.root.join(format!("existing_{i}.txt")),
                format!("existing {i}"),
            )
            .unwrap();
        }
        repo.git(&["add", "."]).unwrap();
        repo.git(&["commit", "-m", "add existing files"]).unwrap();

        // Move some existing files
        fs::create_dir_all(repo.root.join("moved")).unwrap();
        for i in 0..3 {
            repo.git(&[
                "mv",
                &format!("existing_{i}.txt"),
                &format!("moved/existing_{i}.txt"),
            ])
            .unwrap();
        }

        // Delete some existing files
        repo.git(&["rm", "existing_3.txt"]).unwrap();

        // Add brand new files (never committed)
        for i in 0..5 {
            fs::write(
                repo.root.join(format!("brand_new_{i}.txt")),
                format!("new {i}"),
            )
            .unwrap();
        }
        repo.git(&["add", "."]).unwrap();

        // Capture statuses — includes both committed moves AND new files
        let statuses = repo.file_statuses().unwrap();

        // Reset head
        repo.reset_head().unwrap();

        // Stage each file — new files should still be on disk and stageable
        let mut failed = Vec::new();
        for (file, status) in &statuses {
            if file == "init.txt" {
                continue;
            }
            let ok = repo.stage_file(file).unwrap();
            if !ok {
                failed.push((file.clone(), *status));
            }
        }

        assert!(
            failed.is_empty(),
            "stage_file failed for {} files: {:?}",
            failed.len(),
            failed
        );
    }

    /// Regression: git status --porcelain C-quotes paths that contain
    /// spaces or non-ASCII characters.  file_statuses() must unquote
    /// them so that stage_file receives real filesystem paths, not
    /// quoted strings that git add cannot resolve.
    #[test]
    fn stage_file_handles_quoted_paths_from_moves() {
        let (_dir, repo) = temp_repo();

        // Create and commit a file with spaces in the name
        fs::write(repo.root.join("old name.txt"), "content").unwrap();
        repo.git(&["add", "."]).unwrap();
        repo.git(&["commit", "-m", "add file with spaces"]).unwrap();

        // Move it (git mv)
        repo.git(&["mv", "old name.txt", "new name.txt"]).unwrap();

        // file_statuses must return unquoted paths
        let statuses = repo.file_statuses().unwrap();

        // The paths should NOT have C-quotes
        assert!(
            statuses.contains_key("old name.txt"),
            "old path should be unquoted; got keys: {:?}",
            statuses.keys().collect::<Vec<_>>()
        );
        assert!(
            statuses.contains_key("new name.txt"),
            "new path should be unquoted; got keys: {:?}",
            statuses.keys().collect::<Vec<_>>()
        );

        // After reset, stage_file must succeed for both sides
        repo.reset_head().unwrap();

        let old_ok = repo.stage_file("old name.txt").unwrap();
        assert!(old_ok, "stage_file should succeed for old (deleted) path");

        let new_ok = repo.stage_file("new name.txt").unwrap();
        assert!(new_ok, "stage_file should succeed for new (added) path");
    }

    /// Regression: ensure file_statuses unquotes C-style paths for
    /// non-rename entries too (modified, deleted, added files with spaces).
    #[test]
    fn file_statuses_unquotes_paths_with_special_chars() {
        let (_dir, repo) = temp_repo();

        // Create files with spaces
        fs::write(repo.root.join("my file.txt"), "content").unwrap();
        fs::write(repo.root.join("to delete.txt"), "delete me").unwrap();
        repo.git(&["add", "."]).unwrap();
        repo.git(&["commit", "-m", "add spaced files"]).unwrap();

        // Modify one, delete another, add a new one with spaces
        fs::write(repo.root.join("my file.txt"), "modified").unwrap();
        repo.git(&["rm", "to delete.txt"]).unwrap();
        fs::write(repo.root.join("brand new file.txt"), "new").unwrap();
        repo.git(&["add", "."]).unwrap();

        let statuses = repo.file_statuses().unwrap();

        // All paths should be unquoted
        assert!(
            statuses.contains_key("my file.txt"),
            "modified file should be unquoted; keys: {:?}",
            statuses.keys().collect::<Vec<_>>()
        );
        assert!(
            statuses.contains_key("to delete.txt"),
            "deleted file should be unquoted; keys: {:?}",
            statuses.keys().collect::<Vec<_>>()
        );
        assert!(
            statuses.contains_key("brand new file.txt"),
            "new file should be unquoted; keys: {:?}",
            statuses.keys().collect::<Vec<_>>()
        );
    }

    /// Test that stage_file works for moved files split across multiple
    /// commits (simulating execute_plan with multiple commits where moves
    /// are split: new path in one commit, old path deletion in another).
    #[test]
    fn stage_file_works_across_sequential_commits_with_moves() {
        let (_dir, repo) = temp_repo();

        // Create and commit files
        for i in 0..10 {
            fs::write(
                repo.root.join(format!("src_{i}.txt")),
                format!("content {i}"),
            )
            .unwrap();
        }
        repo.git(&["add", "."]).unwrap();
        repo.git(&["commit", "-m", "add source files"]).unwrap();

        // Move all files to a new directory
        fs::create_dir_all(repo.root.join("dst")).unwrap();
        for i in 0..10 {
            repo.git(&["mv", &format!("src_{i}.txt"), &format!("dst/src_{i}.txt")])
                .unwrap();
        }

        let statuses = repo.file_statuses().unwrap();
        repo.reset_head().unwrap();

        // Commit 1: stage the NEW paths (additions)
        for i in 0..10 {
            let file = format!("dst/src_{i}.txt");
            let ok = repo.stage_file(&file).unwrap();
            assert!(ok, "should stage new path {file}");
        }
        repo.commit("feat: add new paths").unwrap();

        // Commit 2: stage the OLD paths (deletions) — these must still work
        // even though HEAD has changed after commit 1
        let mut failed = Vec::new();
        for i in 0..10 {
            let file = format!("src_{i}.txt");
            if let Some(&status) = statuses.get(&file) {
                let ok = repo.stage_file(&file).unwrap();
                if !ok {
                    failed.push((file, status));
                }
            }
        }

        assert!(
            failed.is_empty(),
            "stage_file failed for old paths after prior commit: {:?}",
            failed
        );
    }
}
