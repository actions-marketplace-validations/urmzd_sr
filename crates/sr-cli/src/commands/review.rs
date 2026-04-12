use anyhow::{Result, bail};
use sr_core::ai::BackendConfig;
use sr_core::ai::git::GitRepo;
use sr_core::ai::services::review::{self, PrReviewInput};
use sr_core::native_git::NativeGitRepository;
use sr_core::github::GitHubProvider;
use std::path::Path;

use super::ui;

#[derive(Debug, clap::Args)]
pub struct ReviewArgs {
    /// Additional context or instructions for the review
    #[arg(short = 'M', long)]
    pub message: Option<String>,

    /// Post the review as a comment on the GitHub PR
    #[arg(long)]
    pub comment: bool,
}

pub async fn run(args: &ReviewArgs, backend_config: &BackendConfig) -> Result<()> {
    let repo = GitRepo::discover()?;

    // Build GitHub provider
    let git = NativeGitRepository::open(Path::new("."))?;
    let (hostname, owner, repo_name) = git.parse_remote_full()?;

    let token = std::env::var("GH_TOKEN")
        .or_else(|_| std::env::var("GITHUB_TOKEN"))
        .map_err(|_| anyhow::anyhow!("neither GH_TOKEN nor GITHUB_TOKEN is set — required for PR review"))?;

    let github = GitHubProvider::new(owner, repo_name, hostname, token);

    // Get current branch
    let branch = repo.current_branch()?;

    let spinner = ui::spinner(&format!("Finding PR for branch '{branch}'..."));

    // Find PR for current branch
    let pr_meta = github
        .get_pr_for_branch(&branch)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    // Fetch diff
    let diff = github
        .get_pr_diff(pr_meta.number)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    ui::spinner_done(
        &spinner,
        Some(&format!("PR #{} — {}", pr_meta.number, pr_meta.title)),
    );

    if diff.trim().is_empty() {
        bail!("PR #{} has no changes to review", pr_meta.number);
    }

    let spinner = ui::spinner("Reviewing PR...");

    let input = PrReviewInput {
        title: &pr_meta.title,
        description: pr_meta.body.as_deref(),
        author: &pr_meta.user.login,
        base: &pr_meta.base.ref_name,
        head: &pr_meta.head.ref_name,
        diff: &diff,
        message: args.message.as_deref(),
    };

    let review_text = review::review_pr(&repo, &input, backend_config).await?;
    spinner.finish_and_clear();

    println!("{review_text}");

    if args.comment {
        let spinner = ui::spinner("Posting review on GitHub...");
        github
            .post_pr_review(pr_meta.number, &review_text)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        ui::spinner_done(&spinner, Some("Review posted"));
    }

    Ok(())
}
