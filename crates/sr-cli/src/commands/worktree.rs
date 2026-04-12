use anyhow::{Result, bail};
use sr_core::git::GitRepo;
use std::io::{self, Write};
use std::path::Path;

#[derive(Debug, clap::Args)]
pub struct WorktreeArgs {
    /// Branch name for the new worktree
    pub branch: String,

    /// Skip confirmation prompts
    #[arg(short, long)]
    pub yes: bool,
}

pub async fn run(args: &WorktreeArgs) -> Result<()> {
    let repo = GitRepo::discover()?;
    let repo_root = repo.root().to_path_buf();
    let has_changes = repo.has_any_changes()?;

    let repo_dir_name = repo_root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("repo");
    let worktree_dir_name = format!("{repo_dir_name}-{}", args.branch);
    let worktree_path = repo_root
        .parent()
        .unwrap_or(Path::new("."))
        .join(&worktree_dir_name);

    if worktree_path.exists() {
        bail!("worktree path already exists: {}", worktree_path.display());
    }

    if has_changes && !args.yes {
        eprintln!("uncommitted changes will be stashed and moved to the new worktree.");
        eprint!("continue? [y/N] ");
        io::stderr().flush()?;

        let mut answer = String::new();
        io::stdin().read_line(&mut answer)?;
        if !answer.trim().eq_ignore_ascii_case("y") {
            bail!("cancelled");
        }
    }

    // Stash changes if any
    let stashed = if has_changes {
        let output = std::process::Command::new("git")
            .args(["-C", &repo_root.to_string_lossy()])
            .args(["stash", "push", "--include-untracked", "-m", "sr worktree: moving changes"])
            .output()?;
        output.status.success()
    } else {
        false
    };

    // Create worktree
    let status = std::process::Command::new("git")
        .args(["-C", &repo_root.to_string_lossy()])
        .args(["worktree", "add", "-b", &args.branch, &worktree_path.to_string_lossy()])
        .status()?;

    if !status.success() {
        if stashed {
            let _ = std::process::Command::new("git")
                .args(["-C", &repo_root.to_string_lossy()])
                .args(["stash", "pop"])
                .status();
        }
        bail!("failed to create worktree");
    }

    // Apply stashed changes
    if stashed {
        let pop = std::process::Command::new("git")
            .args(["-C", &worktree_path.to_string_lossy()])
            .args(["stash", "pop"])
            .output()?;

        if !pop.status.success() {
            let stderr = String::from_utf8_lossy(&pop.stderr);
            eprintln!("warning: failed to apply changes: {}", stderr.trim());
            eprintln!("your changes are in the stash — run `git stash pop` in the worktree");
        }
    }

    eprintln!("worktree created: {}", worktree_path.display());
    eprintln!("  cd {}", worktree_path.display());

    Ok(())
}
