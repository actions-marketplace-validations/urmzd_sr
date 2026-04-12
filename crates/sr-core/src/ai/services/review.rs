use crate::ai::backend::{AiRequest, BackendConfig, resolve_backend};
use crate::ai::git::GitRepo;
use anyhow::{Result, bail};

pub struct ReviewInput<'a> {
    pub diff: &'a str,
    pub message: Option<&'a str>,
}

/// Review a local diff and return the review text.
pub async fn review(
    repo: &GitRepo,
    input: &ReviewInput<'_>,
    backend_config: &BackendConfig,
) -> Result<String> {
    if input.diff.trim().is_empty() {
        bail!("no changes to review");
    }

    let backend = resolve_backend(backend_config).await?;

    let mut user_prompt = format!("Review this diff:\n\n{}", input.diff);
    if let Some(msg) = input.message {
        user_prompt.push_str(&format!("\n\nAdditional context: {msg}"));
    }

    let request = AiRequest {
        system_prompt: crate::ai::prompts::review::SYSTEM_PROMPT.to_string(),
        user_prompt,
        json_schema: None,
        working_dir: repo.root().to_string_lossy().to_string(),
    };

    let response = backend.request(&request, None).await?;
    Ok(response.text)
}

pub struct PrReviewInput<'a> {
    pub title: &'a str,
    pub description: Option<&'a str>,
    pub author: &'a str,
    pub base: &'a str,
    pub head: &'a str,
    pub diff: &'a str,
    pub message: Option<&'a str>,
}

/// Review a GitHub PR diff and return the review text.
pub async fn review_pr(
    repo: &GitRepo,
    input: &PrReviewInput<'_>,
    backend_config: &BackendConfig,
) -> Result<String> {
    if input.diff.trim().is_empty() {
        bail!("PR has no changes to review");
    }

    let backend = resolve_backend(backend_config).await?;

    let mut user_prompt = format!(
        "Review this pull request:\n\n\
         Title: {}\n\
         Author: {}\n\
         Base: {} ← {}\n",
        input.title, input.author, input.base, input.head
    );

    if let Some(desc) = input.description {
        if !desc.is_empty() {
            user_prompt.push_str(&format!("\nDescription:\n{desc}\n"));
        }
    }

    user_prompt.push_str(&format!("\nDiff:\n{}", input.diff));

    if let Some(msg) = input.message {
        user_prompt.push_str(&format!("\n\nAdditional reviewer context: {msg}"));
    }

    let request = AiRequest {
        system_prompt: crate::ai::prompts::review::PR_SYSTEM_PROMPT.to_string(),
        user_prompt,
        json_schema: None,
        working_dir: repo.root().to_string_lossy().to_string(),
    };

    let response = backend.request(&request, None).await?;
    Ok(response.text)
}
