use anyhow::Result;
use sr_core::ai::BackendConfig;
use sr_core::ai::git::GitRepo;
use sr_core::ai::services::pr::{self, PrInput};

use super::ui;

#[derive(Debug, clap::Args)]
pub struct PrArgs {
    /// Additional context or instructions for PR generation
    #[arg(short = 'M', long)]
    pub message: Option<String>,

    /// Draft PR
    #[arg(short, long)]
    pub draft: bool,
}

pub async fn run(args: &PrArgs, backend_config: &BackendConfig) -> Result<()> {
    let repo = GitRepo::discover()?;

    // Auto-detect base branch from sr.yaml config, fallback to "main"
    let config = sr_core::config::ReleaseConfig::find_config(repo.root().as_path())
        .map(|(path, _)| sr_core::config::ReleaseConfig::load(&path))
        .transpose()?
        .unwrap_or_default();
    let base = config
        .branches
        .first()
        .cloned()
        .unwrap_or_else(|| "main".to_string());

    let spinner = ui::spinner("Generating PR...");

    let input = PrInput {
        base: &base,
        message: args.message.as_deref(),
    };
    let content = pr::generate(&repo, &input, backend_config).await?;
    spinner.finish_and_clear();

    println!("Title: {}", content.title);
    println!();
    println!("{}", content.body);

    // Always create/update the PR
    let mut cmd = std::process::Command::new("gh");
    cmd.args([
        "pr", "create", "--title", &content.title, "--body", &content.body, "--base", &base,
    ]);

    if args.draft {
        cmd.arg("--draft");
    }

    cmd.current_dir(repo.root()).status()?;

    Ok(())
}
