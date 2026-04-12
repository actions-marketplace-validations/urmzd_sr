use crate::ai::backend::{AiRequest, BackendConfig, resolve_backend};
use crate::ai::git::GitRepo;
use anyhow::Result;

pub struct BranchInput<'a> {
    pub context: Option<&'a str>,
}

/// Suggest a conventional branch name based on context or current changes.
pub async fn suggest(
    repo: &GitRepo,
    input: &BranchInput<'_>,
    backend_config: &BackendConfig,
) -> Result<String> {
    let backend = resolve_backend(backend_config).await?;

    let prompt = if let Some(ctx) = input.context {
        format!("Suggest a branch name for: {ctx}")
    } else {
        let status = repo.status_porcelain()?;
        let diff = repo.diff_head()?;
        format!(
            "Based on these changes, suggest a branch name:\n\nStatus:\n{status}\n\nDiff:\n{diff}"
        )
    };

    let request = AiRequest {
        system_prompt: crate::ai::prompts::branch::SYSTEM_PROMPT.to_string(),
        user_prompt: prompt,
        json_schema: None,
        working_dir: repo.root().to_string_lossy().to_string(),
    };

    let response = backend.request(&request, None).await?;
    Ok(response.text.trim().to_string())
}
