use crate::ai::{AiRequest, BackendConfig, resolve_backend};
use crate::git::GitRepo;
use crate::ui;
use anyhow::Result;

#[derive(Debug, clap::Args)]
pub struct AskArgs {
    /// Question to ask about the repo
    pub question: Vec<String>,
}

pub async fn run(args: &AskArgs, backend_config: &BackendConfig) -> Result<()> {
    let repo = GitRepo::discover()?;
    let backend = resolve_backend(backend_config).await?;

    let question = args.question.join(" ");
    if question.is_empty() {
        anyhow::bail!("please provide a question");
    }

    let spinner = ui::spinner(&format!("Thinking with {}...", backend.name()));

    let request = AiRequest {
        system_prompt: crate::prompts::ask::SYSTEM_PROMPT.to_string(),
        user_prompt: question,
        json_schema: None,
        working_dir: repo.root().to_string_lossy().to_string(),
    };

    let response = backend.request(&request, None).await?;
    spinner.finish_and_clear();

    println!("{}", response.text);

    Ok(())
}
