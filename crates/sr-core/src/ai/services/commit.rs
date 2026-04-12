use crate::ai::backend::{AiEvent, AiRequest, AiUsage, BackendConfig, resolve_backend};
use crate::ai::cache::{CacheLookup, CacheManager};
use crate::ai::git::{GitRepo, SnapshotGuard};
use anyhow::{Context, Result, bail};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;

use crate::ai::prompts;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitPlan {
    pub commits: Vec<PlannedCommit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedCommit {
    pub order: Option<u32>,
    pub message: String,
    pub body: Option<String>,
    pub footer: Option<String>,
    pub files: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum CacheStatus {
    None,
    Cached,
    Patched,
    PatchedWithAi,
}

pub enum CommitOutcome {
    Created { sha: String, message: String },
    Skipped { message: String },
    Failed { index: usize, message: String, error: String },
}

// ---------------------------------------------------------------------------
// Plan generation
// ---------------------------------------------------------------------------

pub struct PlanInput<'a> {
    pub staged_only: bool,
    pub message: Option<&'a str>,
    pub no_cache: bool,
    pub commit_pattern: &'a str,
    pub type_names: &'a [&'a str],
}

pub struct PlanResult<'a> {
    pub plan: CommitPlan,
    pub cache_status: CacheStatus,
    pub statuses: HashMap<String, char>,
    pub snapshot: SnapshotGuard<'a>,
}

/// Event callback for AI tool calls during plan generation.
pub type EventSender = mpsc::UnboundedSender<AiEvent>;

pub struct PlanMetrics {
    pub backend_name: String,
    pub model_name: String,
    pub file_count: usize,
    pub usage: Option<AiUsage>,
}

/// Generate a commit plan from repository changes.
///
/// Returns the plan, cache status, file statuses, a snapshot guard,
/// and metrics about the AI call. The caller is responsible for:
/// - Displaying progress (spinners, status messages)
/// - Confirming with the user
/// - Calling `execute_plan` to apply the commits
/// - Calling `snapshot.success()` when done
pub async fn generate_plan<'a>(
    repo: &'a GitRepo,
    input: &PlanInput<'_>,
    backend_config: &BackendConfig,
    event_tx: Option<EventSender>,
) -> Result<(PlanResult<'a>, PlanMetrics)> {
    let system_prompt = prompts::commit::system_prompt(input.commit_pattern, input.type_names);

    // Check for changes
    let has_changes = if input.staged_only {
        repo.has_staged_changes()?
    } else {
        repo.has_any_changes()?
    };

    if !has_changes {
        bail!(crate::ai::error::SrAiError::NoChanges);
    }

    let statuses = repo.file_statuses().unwrap_or_default();
    let file_count = statuses.len();

    // Resolve AI backend
    let backend = resolve_backend(backend_config).await?;
    let backend_name = backend.name().to_string();
    let model_name = backend_config
        .model
        .as_deref()
        .unwrap_or("default")
        .to_string();

    // Build cache manager
    let cache = if input.no_cache {
        None
    } else {
        CacheManager::new(
            repo.root(),
            input.staged_only,
            input.message,
            &backend_name,
            &model_name,
        )
    };

    // Snapshot the working tree
    let snapshot = SnapshotGuard::new(repo)?;

    // Generate plan (cache or AI)
    let (mut plan, cache_status, usage) = match cache.as_ref().map(|c| c.lookup()) {
        Some(CacheLookup::ExactHit(cached_plan)) => {
            (cached_plan, CacheStatus::Cached, None)
        }
        Some(CacheLookup::PatchHit {
            plan: patched_plan,
            unplaced_files,
            delta_summary,
            ..
        }) => {
            if unplaced_files.is_empty() {
                (patched_plan, CacheStatus::Patched, None)
            } else {
                let user_prompt = prompts::commit::patch_prompt(
                    input.staged_only,
                    &repo.root().to_string_lossy(),
                    input.message,
                    &patched_plan,
                    &unplaced_files,
                    &delta_summary,
                );

                let request = AiRequest {
                    system_prompt: system_prompt.clone(),
                    user_prompt,
                    json_schema: Some(prompts::commit::SCHEMA.to_string()),
                    working_dir: repo.root().to_string_lossy().to_string(),
                };

                let response = backend.request(&request, event_tx.clone()).await?;
                let p: CommitPlan = parse_plan(&response.text)?;
                (p, CacheStatus::PatchedWithAi, response.usage)
            }
        }
        _ => {
            let user_prompt = prompts::commit::user_prompt(
                input.staged_only,
                &repo.root().to_string_lossy(),
                input.message,
            );

            let request = AiRequest {
                system_prompt: system_prompt.clone(),
                user_prompt,
                json_schema: Some(prompts::commit::SCHEMA.to_string()),
                working_dir: repo.root().to_string_lossy().to_string(),
            };

            let response = backend.request(&request, event_tx).await?;
            let p: CommitPlan = parse_plan(&response.text)?;
            (p, CacheStatus::None, response.usage)
        }
    };

    if plan.commits.is_empty() {
        bail!(crate::ai::error::SrAiError::EmptyPlan);
    }

    // Validate: merge commits with shared files
    plan = validate_plan(plan);

    // Store in cache
    if let Some(cache) = &cache {
        cache.store(&plan, &backend_name, &model_name);
    }

    let result = PlanResult {
        plan,
        cache_status,
        statuses,
        snapshot,
    };

    let metrics = PlanMetrics {
        backend_name,
        model_name,
        file_count,
        usage,
    };

    Ok((result, metrics))
}

// ---------------------------------------------------------------------------
// Plan execution
// ---------------------------------------------------------------------------

/// Execute a commit plan, returning outcomes for each commit.
pub fn execute_plan(repo: &GitRepo, plan: &CommitPlan) -> Result<Vec<CommitOutcome>> {
    repo.reset_head()?;

    let mut outcomes = Vec::new();

    for (i, commit) in plan.commits.iter().enumerate() {
        // Stage files
        for file in &commit.files {
            repo.stage_file(file)?;
        }

        // Build full commit message
        let mut full_message = commit.message.clone();
        if let Some(body) = &commit.body {
            if !body.is_empty() {
                full_message.push_str("\n\n");
                full_message.push_str(body);
            }
        }
        if let Some(footer) = &commit.footer {
            if !footer.is_empty() {
                full_message.push_str("\n\n");
                full_message.push_str(footer);
            }
        }

        if repo.has_staged_after_add()? {
            match repo.commit(&full_message) {
                Ok(()) => {
                    let sha = repo.head_short().unwrap_or_else(|_| "???????".to_string());
                    outcomes.push(CommitOutcome::Created {
                        sha,
                        message: commit.message.clone(),
                    });
                }
                Err(e) => {
                    outcomes.push(CommitOutcome::Failed {
                        index: i + 1,
                        message: commit.message.clone(),
                        error: format!("{e:#}"),
                    });
                    repo.reset_head()?;
                }
            }
        } else {
            outcomes.push(CommitOutcome::Skipped {
                message: commit.message.clone(),
            });
        }
    }

    Ok(outcomes)
}

// ---------------------------------------------------------------------------
// Pure helpers
// ---------------------------------------------------------------------------

/// Validate that no file appears in multiple commits. If duplicates are found,
/// merge affected commits into one.
pub fn validate_plan(plan: CommitPlan) -> CommitPlan {
    let mut file_counts: HashMap<String, usize> = HashMap::new();
    for commit in &plan.commits {
        for file in &commit.files {
            *file_counts.entry(file.clone()).or_default() += 1;
        }
    }

    let dupes: Vec<&String> = file_counts
        .iter()
        .filter(|(_, count)| **count > 1)
        .map(|(file, _)| file)
        .collect();

    if dupes.is_empty() {
        return plan;
    }

    let mut tainted = Vec::new();
    let mut clean = Vec::new();

    for commit in plan.commits {
        let is_tainted = commit.files.iter().any(|f| dupes.contains(&f));
        if is_tainted {
            tainted.push(commit);
        } else {
            clean.push(commit);
        }
    }

    let merged_message = tainted
        .first()
        .map(|c| c.message.clone())
        .unwrap_or_default();

    let merged_body = tainted
        .iter()
        .filter_map(|c| c.body.as_ref())
        .filter(|b| !b.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join("\n\n");

    let merged_footer = tainted
        .iter()
        .filter_map(|c| c.footer.as_ref())
        .filter(|f| !f.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");

    let mut merged_files: Vec<String> = tainted
        .iter()
        .flat_map(|c| c.files.iter().cloned())
        .collect();
    merged_files.sort();
    merged_files.dedup();

    let merged_commit = PlannedCommit {
        order: Some(1),
        message: merged_message,
        body: if merged_body.is_empty() {
            None
        } else {
            Some(merged_body)
        },
        footer: if merged_footer.is_empty() {
            None
        } else {
            Some(merged_footer)
        },
        files: merged_files,
    };

    let mut result = vec![merged_commit];
    for (i, mut commit) in clean.into_iter().enumerate() {
        commit.order = Some(i as u32 + 2);
        result.push(commit);
    }

    CommitPlan { commits: result }
}

/// Parse a commit plan from JSON text, tolerating duplicate fields.
pub fn parse_plan(text: &str) -> Result<CommitPlan> {
    let value: serde_json::Value =
        serde_json::from_str(text).context("failed to parse JSON from AI response")?;
    serde_json::from_value(value).context("failed to parse commit plan from AI response")
}

/// Validate that all commit messages match the configured pattern.
pub fn validate_messages(plan: &CommitPlan, commit_pattern: &str) -> Vec<(usize, String, String)> {
    let re = match Regex::new(commit_pattern) {
        Ok(re) => re,
        Err(e) => {
            return plan
                .commits
                .iter()
                .enumerate()
                .map(|(i, c)| (i + 1, c.message.clone(), format!("invalid pattern: {e}")))
                .collect();
        }
    };

    plan.commits
        .iter()
        .enumerate()
        .filter(|(_, c)| !re.is_match(&c.message))
        .map(|(i, c)| {
            (
                i + 1,
                c.message.clone(),
                format!("does not match pattern: {commit_pattern}"),
            )
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_plan_no_dupes() {
        let plan = CommitPlan {
            commits: vec![
                PlannedCommit {
                    order: Some(1),
                    message: "feat: add foo".into(),
                    body: Some("reason".into()),
                    footer: None,
                    files: vec!["a.rs".into()],
                },
                PlannedCommit {
                    order: Some(2),
                    message: "fix: fix bar".into(),
                    body: Some("reason".into()),
                    footer: None,
                    files: vec!["b.rs".into()],
                },
            ],
        };

        let result = validate_plan(plan);
        assert_eq!(result.commits.len(), 2);
    }

    #[test]
    fn validate_plan_merges_dupes() {
        let plan = CommitPlan {
            commits: vec![
                PlannedCommit {
                    order: Some(1),
                    message: "feat: add foo".into(),
                    body: Some("reason 1".into()),
                    footer: None,
                    files: vec!["shared.rs".into(), "a.rs".into()],
                },
                PlannedCommit {
                    order: Some(2),
                    message: "fix: fix bar".into(),
                    body: Some("reason 2".into()),
                    footer: None,
                    files: vec!["shared.rs".into(), "b.rs".into()],
                },
                PlannedCommit {
                    order: Some(3),
                    message: "docs: update readme".into(),
                    body: Some("docs".into()),
                    footer: None,
                    files: vec!["README.md".into()],
                },
            ],
        };

        let result = validate_plan(plan);
        assert_eq!(result.commits.len(), 2);
        assert_eq!(result.commits[0].message, "feat: add foo");
        assert!(result.commits[0].files.contains(&"shared.rs".to_string()));
        assert!(result.commits[0].files.contains(&"a.rs".to_string()));
        assert!(result.commits[0].files.contains(&"b.rs".to_string()));
        assert_eq!(result.commits[1].message, "docs: update readme");
        assert_eq!(result.commits[1].order, Some(2));
    }

    #[test]
    fn validate_messages_all_valid() {
        let plan = CommitPlan {
            commits: vec![
                PlannedCommit {
                    order: Some(1),
                    message: "feat: add foo".into(),
                    body: None,
                    footer: None,
                    files: vec![],
                },
                PlannedCommit {
                    order: Some(2),
                    message: "fix(core): null check".into(),
                    body: None,
                    footer: None,
                    files: vec![],
                },
            ],
        };

        let pattern = crate::commit::DEFAULT_COMMIT_PATTERN;
        let invalid = validate_messages(&plan, pattern);
        assert!(invalid.is_empty());
    }

    #[test]
    fn validate_messages_catches_invalid() {
        let plan = CommitPlan {
            commits: vec![
                PlannedCommit {
                    order: Some(1),
                    message: "feat: add foo".into(),
                    body: None,
                    footer: None,
                    files: vec![],
                },
                PlannedCommit {
                    order: Some(2),
                    message: "not a conventional commit".into(),
                    body: None,
                    footer: None,
                    files: vec![],
                },
                PlannedCommit {
                    order: Some(3),
                    message: "fix: valid one".into(),
                    body: None,
                    footer: None,
                    files: vec![],
                },
            ],
        };

        let pattern = crate::commit::DEFAULT_COMMIT_PATTERN;
        let invalid = validate_messages(&plan, pattern);
        assert_eq!(invalid.len(), 1);
        assert_eq!(invalid[0].0, 2);
        assert_eq!(invalid[0].1, "not a conventional commit");
    }

    #[test]
    fn validate_messages_invalid_pattern() {
        let plan = CommitPlan {
            commits: vec![PlannedCommit {
                order: Some(1),
                message: "feat: add foo".into(),
                body: None,
                footer: None,
                files: vec![],
            }],
        };

        let invalid = validate_messages(&plan, "[invalid regex");
        assert_eq!(invalid.len(), 1);
        assert!(invalid[0].2.contains("invalid pattern"));
    }
}
