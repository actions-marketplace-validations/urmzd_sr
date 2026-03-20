use anyhow::{Context, Result, bail};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

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
            bail!(crate::error::SrAiError::NotAGitRepo);
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
            bail!(crate::error::SrAiError::GitCommand(format!(
                "git {} failed: {}",
                args.join(" "),
                stderr.trim()
            )));
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
        let full_path = self.root.join(file);
        let exists = full_path.exists();

        if !exists {
            // Check if it's a deleted file
            let out = self.git(&["ls-files", "--deleted"])?;
            let is_deleted = out.lines().any(|l| l.trim() == file);
            if !is_deleted {
                return Ok(false);
            }
        }

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
            bail!(crate::error::SrAiError::GitCommand(format!(
                "git commit failed: {}",
                stderr.trim()
            )));
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

    pub fn file_statuses(&self) -> Result<HashMap<String, char>> {
        let out = self.git(&["status", "--porcelain"])?;
        let mut map = HashMap::new();
        for line in out.lines() {
            if line.len() < 3 {
                continue;
            }
            let xy = &line.as_bytes()[..2];
            let mut path = line[3..].to_string();
            if let Some(pos) = path.find(" -> ") {
                path = path[pos + 4..].to_string();
            }
            let (x, y) = (xy[0], xy[1]);
            let status = match (x, y) {
                (b'?', b'?') => 'A',
                (b'A', _) | (_, b'A') => 'A',
                (b'D', _) | (_, b'D') => 'D',
                (b'R', _) | (_, b'R') => 'R',
                (b'M', _) | (_, b'M') | (b'T', _) | (_, b'T') => 'M',
                _ => '~',
            };
            map.insert(path, status);
        }
        Ok(map)
    }
}
