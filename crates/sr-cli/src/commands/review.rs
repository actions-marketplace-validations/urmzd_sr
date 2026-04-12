use anyhow::{Result, bail};
use sr_core::git::GitRepo;
use sr_core::native_git::NativeGitRepository;
use sr_core::github::GitHubProvider;
use std::path::Path;

#[derive(Debug, clap::Args)]
pub struct ReviewArgs {
    /// Post the diff as a comment on the GitHub PR
    #[arg(long)]
    pub comment: bool,
}

pub async fn run(args: &ReviewArgs) -> Result<()> {
    let repo = GitRepo::discover()?;

    let git = NativeGitRepository::open(Path::new("."))?;
    let (hostname, owner, repo_name) = git.parse_remote_full()?;

    let token = std::env::var("GH_TOKEN")
        .or_else(|_| std::env::var("GITHUB_TOKEN"))
        .map_err(|_| anyhow::anyhow!("GH_TOKEN or GITHUB_TOKEN required for PR review"))?;

    let github = GitHubProvider::new(owner, repo_name, hostname, token);
    let branch = repo.current_branch()?;

    let pr_meta = github
        .get_pr_for_branch(&branch)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let diff = github
        .get_pr_diff(pr_meta.number)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    if diff.trim().is_empty() {
        bail!("PR #{} has no changes", pr_meta.number);
    }

    println!("PR #{} — {}", pr_meta.number, pr_meta.title);
    println!();
    println!("{diff}");

    if args.comment {
        eprintln!("note: AI review not available. Use sr via MCP for AI-powered reviews.");
    }

    Ok(())
}
