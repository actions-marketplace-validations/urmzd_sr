use anyhow::{Result, bail};
use sr_core::ai::BackendConfig;
use sr_core::ai::git::GitRepo;
use sr_core::ai::services::rebase::{self, RebaseInput};

use super::ui;

#[derive(Debug, clap::Args)]
pub struct RebaseArgs {
    /// Additional context or instructions for reorganization
    #[arg(short = 'M', long)]
    pub message: Option<String>,

    /// Display plan without executing
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Skip confirmation prompt
    #[arg(short, long)]
    pub yes: bool,

    /// Number of recent commits to reorganize (default: auto-detect since last tag)
    #[arg(long)]
    pub last: Option<usize>,
}

pub async fn run(args: &RebaseArgs, backend_config: &BackendConfig) -> Result<()> {
    ui::header("sr rebase");

    let repo = GitRepo::discover()?;
    ui::phase_ok("Repository found", None);

    // Load config for commit pattern/types
    let config = sr_core::config::ReleaseConfig::find_config(repo.root().as_path())
        .map(|(path, _)| sr_core::config::ReleaseConfig::load(&path))
        .transpose()?
        .unwrap_or_default();
    let type_names: Vec<&str> = config.types.iter().map(|t| t.name.as_str()).collect();

    // Determine commit count
    let commit_count = match args.last {
        Some(n) => n,
        None => {
            let count = repo.commits_since_last_tag()?;
            if count == 0 {
                bail!("no commits found to rebase");
            }
            count
        }
    };
    ui::phase_ok("Commits loaded", Some(&format!("{commit_count} commits")));

    // Generate plan via service
    let spinner = ui::spinner("Analyzing commits...");
    let (tx, event_handler) = ui::spawn_event_handler(&spinner);

    let input = RebaseInput {
        message: args.message.as_deref(),
        commit_count,
        commit_pattern: &config.commit_pattern,
        type_names: &type_names,
    };

    let (plan, metrics) =
        rebase::generate_plan(&repo, &input, backend_config, Some(tx)).await?;
    let _ = event_handler.await;

    let detail = ui::format_done_detail(plan.commits.len(), "", &metrics.usage);
    ui::spinner_done(&spinner, Some(&detail));

    // Display plan
    ui::display_rebase_plan(&plan);

    if args.dry_run {
        ui::info("Dry run — no changes made");
        println!();
        return Ok(());
    }

    if !args.yes && !ui::confirm("Execute rebase? [y/N]")? {
        bail!(sr_core::ai::error::SrAiError::Cancelled);
    }

    // Execute
    ui::info(&format!("Rebasing {commit_count} commits..."));
    rebase::execute_rebase(&repo, &plan, commit_count)?;

    // Show new history
    let new_log = repo.recent_commits(commit_count)?;
    println!();
    ui::phase_ok("Rebase complete", None);
    println!();
    for line in new_log.lines() {
        println!("    {line}");
    }
    println!();

    Ok(())
}
