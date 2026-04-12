use crate::ai::backend::{AiRequest, BackendConfig, resolve_backend};
use crate::ai::git::GitRepo;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct PrContent {
    pub title: String,
    pub body: String,
}

pub struct PrInput<'a> {
    pub base: &'a str,
    pub message: Option<&'a str>,
}

/// Generate a PR title and body from branch commits.
pub async fn generate(
    repo: &GitRepo,
    input: &PrInput<'_>,
    backend_config: &BackendConfig,
) -> Result<PrContent> {
    let backend = resolve_backend(backend_config).await?;

    let branch = repo.current_branch()?;
    let log = repo.log_range(&format!("{}..HEAD", input.base), None)?;
    let diff = repo.diff_range(input.base)?;

    let mut user_prompt = format!(
        "Generate a PR title and body for branch '{branch}' targeting '{}'.\n\n\
         Commits:\n{log}\n\nDiff:\n{diff}",
        input.base
    );
    if let Some(msg) = input.message {
        user_prompt.push_str(&format!("\n\nAdditional context: {msg}"));
    }

    let request = AiRequest {
        system_prompt: crate::ai::prompts::pr::SYSTEM_PROMPT.to_string(),
        user_prompt,
        json_schema: Some(crate::ai::prompts::pr::SCHEMA.to_string()),
        working_dir: repo.root().to_string_lossy().to_string(),
    };

    let response = backend.request(&request, None).await?;
    let pr: PrContent = serde_json::from_str(&response.text)?;
    Ok(pr)
}
