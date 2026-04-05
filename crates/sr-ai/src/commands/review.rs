use crate::ai::{AiRequest, BackendConfig, resolve_backend};
use crate::git::GitRepo;
use crate::ui;
use anyhow::{Result, bail};

#[derive(Debug, clap::Args)]
pub struct ReviewArgs {
    /// Review staged changes only (default: all changes)
    #[arg(short, long)]
    pub staged: bool,

    /// Base ref to diff against (e.g., main, HEAD~3)
    #[arg(short, long)]
    pub base: Option<String>,
}

pub async fn run(args: &ReviewArgs, backend_config: &BackendConfig) -> Result<()> {
    let repo = GitRepo::discover()?;
    let backend = resolve_backend(backend_config).await?;

    let diff = if let Some(base) = &args.base {
        repo.diff_range(base)?
    } else if args.staged {
        repo.diff_cached()?
    } else {
        repo.diff_head()?
    };

    if diff.trim().is_empty() {
        bail!("no changes to review");
    }

    let spinner = ui::spinner(&format!("Reviewing changes with {}...", backend.name()));

    let request = AiRequest {
        system_prompt: crate::prompts::review::SYSTEM_PROMPT.to_string(),
        user_prompt: format!("Review this diff:\n\n{diff}"),
        json_schema: None,
        working_dir: repo.root().to_string_lossy().to_string(),
    };

    let response = backend.request(&request, None).await?;
    spinner.finish_and_clear();

    println!("{}", response.text);

    Ok(())
}
