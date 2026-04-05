use crate::ai::{AiRequest, BackendConfig, resolve_backend};
use crate::git::GitRepo;
use crate::ui;
use anyhow::Result;

#[derive(Debug, clap::Args)]
pub struct BranchArgs {
    /// Description of what you're working on
    pub description: Option<String>,

    /// Create the branch after suggesting it
    #[arg(short, long)]
    pub create: bool,
}

pub async fn run(args: &BranchArgs, backend_config: &BackendConfig) -> Result<()> {
    let repo = GitRepo::discover()?;
    let backend = resolve_backend(backend_config).await?;

    let prompt = if let Some(desc) = &args.description {
        format!("Suggest a branch name for: {desc}")
    } else {
        let status = repo.status_porcelain()?;
        let diff = repo.diff_head()?;
        format!(
            "Based on these changes, suggest a branch name:\n\nStatus:\n{status}\n\nDiff:\n{diff}"
        )
    };

    let spinner = ui::spinner(&format!(
        "Suggesting branch name with {}...",
        backend.name()
    ));

    let request = AiRequest {
        system_prompt: crate::prompts::branch::SYSTEM_PROMPT.to_string(),
        user_prompt: prompt,
        json_schema: None,
        working_dir: repo.root().to_string_lossy().to_string(),
    };

    let response = backend.request(&request, None).await?;
    spinner.finish_and_clear();

    let branch_name = response.text.trim().to_string();
    println!("{branch_name}");

    if args.create {
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.root().to_string_lossy(),
                "checkout",
                "-b",
                &branch_name,
            ])
            .status()?;
    }

    Ok(())
}
