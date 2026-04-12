use anyhow::{Result, bail};
use sr_core::ai::BackendConfig;
use sr_core::ai::git::GitRepo;
use sr_core::ai::services::commit::{
    self, CacheStatus, CommitOutcome, PlanInput,
};

use super::ui;

#[derive(Debug, clap::Args)]
pub struct CommitArgs {
    /// Additional context or instructions for commit generation
    #[arg(short = 'M', long)]
    pub message: Option<String>,

    /// Display plan without executing
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Skip confirmation prompt
    #[arg(short, long)]
    pub yes: bool,

    /// Bypass cache (always call AI)
    #[arg(long)]
    pub no_cache: bool,

    /// Rebase mode: reorganize existing commits (reword, squash, reorder)
    #[arg(long)]
    pub rebase: bool,

    /// Number of recent commits to reorganize (default: auto-detect since last tag)
    #[arg(long, requires = "rebase")]
    pub last: Option<usize>,
}

pub async fn run(args: &CommitArgs, backend_config: &BackendConfig) -> Result<()> {
    if args.rebase {
        let rebase_args = super::rebase::RebaseArgs {
            message: args.message.clone(),
            dry_run: args.dry_run,
            yes: args.yes,
            last: args.last,
        };
        return super::rebase::run(&rebase_args, backend_config).await;
    }

    ui::header("sr commit");

    // Discover repository
    let repo = GitRepo::discover()?;
    ui::phase_ok("Repository found", None);

    // Load project config
    let config = sr_core::config::ReleaseConfig::find_config(repo.root().as_path())
        .map(|(path, _)| sr_core::config::ReleaseConfig::load(&path))
        .transpose()?
        .unwrap_or_default();
    let type_names: Vec<&str> = config.types.iter().map(|t| t.name.as_str()).collect();

    // Generate plan via service
    let spinner = ui::spinner("Analyzing changes...");
    let (tx, event_handler) = ui::spawn_event_handler(&spinner);

    let input = PlanInput {
        staged_only: false,
        message: args.message.as_deref(),
        no_cache: args.no_cache,
        commit_pattern: &config.commit_pattern,
        type_names: &type_names,
    };

    let (result, metrics) = commit::generate_plan(&repo, &input, backend_config, Some(tx)).await?;
    let _ = event_handler.await;

    let cache_label = match &result.cache_status {
        CacheStatus::Cached => "cached",
        CacheStatus::Patched => "patched",
        CacheStatus::PatchedWithAi => "patched+ai",
        CacheStatus::None => "",
    };
    let detail = ui::format_done_detail(result.plan.commits.len(), cache_label, &metrics.usage);
    ui::spinner_done(&spinner, Some(&detail));

    ui::phase_ok(
        "Changes detected",
        Some(&format!(
            "{} file{}",
            metrics.file_count,
            if metrics.file_count == 1 { "" } else { "s" }
        )),
    );

    // Display plan
    let cache_display: Option<&str> = match &result.cache_status {
        CacheStatus::Cached => Some("cached"),
        CacheStatus::Patched => Some("patched"),
        CacheStatus::PatchedWithAi => Some("patched+ai"),
        CacheStatus::None => None,
    };
    ui::display_plan(&result.plan, &result.statuses, cache_display);

    if args.dry_run {
        ui::info("Dry run — no commits created");
        println!();
        result.snapshot.success();
        return Ok(());
    }

    // Confirm
    if !args.yes && !ui::confirm("Execute plan? [y/N]")? {
        bail!(sr_core::ai::error::SrAiError::Cancelled);
    }

    // Pre-validate commit messages
    let invalid = commit::validate_messages(&result.plan, &config.commit_pattern);
    if !invalid.is_empty() {
        ui::invalid_messages(&invalid);
        if !args.yes && !ui::confirm("Continue anyway? Invalid commits will likely fail. [y/N]")? {
            bail!(sr_core::ai::error::SrAiError::Cancelled);
        }
    }

    // Execute
    let outcomes = commit::execute_plan(&repo, &result.plan)?;

    let mut created = Vec::new();
    let mut failed = Vec::new();
    let total = outcomes.len();

    for (i, outcome) in outcomes.into_iter().enumerate() {
        match outcome {
            CommitOutcome::Created { sha, message } => {
                ui::commit_start(i + 1, total, &message);
                ui::commit_created(&sha);
                created.push((sha, message));
            }
            CommitOutcome::Skipped { message } => {
                ui::commit_start(i + 1, total, &message);
                ui::commit_skipped();
            }
            CommitOutcome::Failed {
                index,
                message,
                error,
            } => {
                ui::commit_start(index, total, &message);
                ui::commit_failed(&error);
                failed.push((index, message, error));
            }
        }
    }

    ui::summary(&created);

    if !failed.is_empty() {
        ui::failed_commits(&failed);
        if created.is_empty() {
            bail!("all {} commits failed", failed.len());
        }
    }

    result.snapshot.success();
    Ok(())
}
