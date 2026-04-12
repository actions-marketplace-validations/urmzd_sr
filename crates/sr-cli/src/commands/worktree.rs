use anyhow::{Result, bail};
use sr_core::ai::BackendConfig;
use sr_core::ai::git::GitRepo;
use sr_core::ai::services::worktree::{self, WorktreeInput};
use std::path::Path;

use super::ui;

#[derive(Debug, clap::Args)]
pub struct WorktreeArgs {
    /// Additional context or instructions for branch naming
    #[arg(short = 'M', long)]
    pub message: Option<String>,
}

pub async fn run(args: &WorktreeArgs, backend_config: &BackendConfig) -> Result<()> {
    let repo = GitRepo::discover()?;
    let repo_root = repo.root().to_path_buf();

    // Check if there are changes to carry over
    let has_changes = repo.has_any_changes()?;

    // Suggest branch name
    let spinner = ui::spinner("Suggesting branch name...");
    let input = WorktreeInput {
        context: args.message.as_deref(),
    };
    let branch_name = worktree::suggest_branch(&repo, &input, backend_config).await?;
    ui::spinner_done(&spinner, Some(&branch_name));

    // Determine worktree path: sibling directory named <repo>-<branch>
    let repo_dir_name = repo_root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("repo");
    let worktree_dir_name = format!("{repo_dir_name}-{branch_name}");
    let worktree_path = repo_root
        .parent()
        .unwrap_or(Path::new("."))
        .join(&worktree_dir_name);

    if worktree_path.exists() {
        bail!(
            "worktree path already exists: {}",
            worktree_path.display()
        );
    }

    // Stash changes if any (so they can be applied in the worktree)
    let stashed = if has_changes {
        let output = std::process::Command::new("git")
            .args(["-C", &repo_root.to_string_lossy()])
            .args(["stash", "push", "--include-untracked", "-m", "sr worktree: moving changes"])
            .output()?;
        output.status.success()
    } else {
        false
    };

    // Create worktree with new branch
    let status = std::process::Command::new("git")
        .args(["-C", &repo_root.to_string_lossy()])
        .args([
            "worktree",
            "add",
            "-b",
            &branch_name,
            &worktree_path.to_string_lossy(),
        ])
        .status()?;

    if !status.success() {
        // Unstash if we stashed
        if stashed {
            let _ = std::process::Command::new("git")
                .args(["-C", &repo_root.to_string_lossy()])
                .args(["stash", "pop"])
                .status();
        }
        bail!("failed to create worktree");
    }

    // Apply stashed changes in the worktree
    if stashed {
        let pop_output = std::process::Command::new("git")
            .args(["-C", &worktree_path.to_string_lossy()])
            .args(["stash", "pop"])
            .output()?;

        if !pop_output.status.success() {
            let stderr = String::from_utf8_lossy(&pop_output.stderr);
            eprintln!("warning: failed to apply changes in worktree: {}", stderr.trim());
            eprintln!("your changes are still in the stash — run `git stash pop` in the worktree");
        }
    }

    ui::phase_ok(
        "Worktree created",
        Some(&worktree_path.to_string_lossy()),
    );
    println!();
    println!("  cd {}", worktree_path.display());
    println!();

    Ok(())
}
