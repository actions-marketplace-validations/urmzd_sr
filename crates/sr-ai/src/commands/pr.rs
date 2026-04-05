use crate::ai::{AiRequest, BackendConfig, resolve_backend};
use crate::git::GitRepo;
use crate::ui;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, clap::Args)]
pub struct PrArgs {
    /// Base branch (default: main)
    #[arg(short, long, default_value = "main")]
    pub base: String,

    /// Create the PR via gh CLI
    #[arg(short, long)]
    pub create: bool,

    /// Draft PR
    #[arg(short, long)]
    pub draft: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct PrOutput {
    title: String,
    body: String,
}

pub async fn run(args: &PrArgs, backend_config: &BackendConfig) -> Result<()> {
    let repo = GitRepo::discover()?;
    let backend = resolve_backend(backend_config).await?;

    let branch = repo.current_branch()?;
    let log = repo.log_range(&format!("{}..HEAD", args.base), None)?;
    let diff = repo.diff_range(&args.base)?;

    let spinner = ui::spinner(&format!("Generating PR with {}...", backend.name()));

    let request = AiRequest {
        system_prompt: crate::prompts::pr::SYSTEM_PROMPT.to_string(),
        user_prompt: format!(
            "Generate a PR title and body for branch '{branch}' targeting '{}'.\n\n\
             Commits:\n{log}\n\nDiff:\n{diff}",
            args.base
        ),
        json_schema: Some(crate::prompts::pr::SCHEMA.to_string()),
        working_dir: repo.root().to_string_lossy().to_string(),
    };

    let response = backend.request(&request, None).await?;
    spinner.finish_and_clear();

    let pr: PrOutput = serde_json::from_str(&response.text)?;

    println!("Title: {}", pr.title);
    println!();
    println!("{}", pr.body);

    if args.create {
        let mut cmd = std::process::Command::new("gh");
        cmd.args([
            "pr", "create", "--title", &pr.title, "--body", &pr.body, "--base", &args.base,
        ]);

        if args.draft {
            cmd.arg("--draft");
        }

        cmd.current_dir(repo.root()).status()?;
    }

    Ok(())
}
