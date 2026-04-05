use crate::ai::{AiRequest, BackendConfig, resolve_backend};
use crate::git::GitRepo;
use crate::ui;
use anyhow::Result;

#[derive(Debug, clap::Args)]
pub struct ExplainArgs {
    /// Commit ref to explain (default: HEAD)
    #[arg(default_value = "HEAD")]
    pub rev: String,
}

pub async fn run(args: &ExplainArgs, backend_config: &BackendConfig) -> Result<()> {
    let repo = GitRepo::discover()?;
    let backend = resolve_backend(backend_config).await?;

    let show = repo.show(&args.rev)?;

    let spinner = ui::spinner(&format!("Explaining commit with {}...", backend.name()));

    let request = AiRequest {
        system_prompt: crate::prompts::explain::SYSTEM_PROMPT.to_string(),
        user_prompt: format!("Explain this commit:\n\n{show}"),
        json_schema: None,
        working_dir: repo.root().to_string_lossy().to_string(),
    };

    let response = backend.request(&request, None).await?;
    spinner.finish_and_clear();

    println!("{}", response.text);

    Ok(())
}
