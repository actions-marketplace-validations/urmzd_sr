use crate::ai::backend::{AiEvent, AiRequest, AiUsage, BackendConfig, resolve_backend};
use crate::ai::git::GitRepo;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;

use crate::ai::prompts;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReorganizePlan {
    pub commits: Vec<ReorganizedCommit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReorganizedCommit {
    pub original_sha: String,
    pub action: String,
    pub message: String,
    pub body: Option<String>,
    pub footer: Option<String>,
}

pub type EventSender = mpsc::UnboundedSender<AiEvent>;

pub struct RebasePlanMetrics {
    pub backend_name: String,
    pub model_name: String,
    pub usage: Option<AiUsage>,
}

// ---------------------------------------------------------------------------
// Plan generation
// ---------------------------------------------------------------------------

pub struct RebaseInput<'a> {
    pub message: Option<&'a str>,
    pub commit_count: usize,
    pub commit_pattern: &'a str,
    pub type_names: &'a [&'a str],
}

/// Generate a rebase plan from recent commits.
pub async fn generate_plan(
    repo: &GitRepo,
    input: &RebaseInput<'_>,
    backend_config: &BackendConfig,
    event_tx: Option<EventSender>,
) -> Result<(ReorganizePlan, RebasePlanMetrics)> {
    if repo.has_any_changes()? {
        bail!("cannot rebase: you have uncommitted changes. Please commit or stash them first.");
    }

    if input.commit_count < 2 {
        bail!(
            "need at least 2 commits to rebase (found {})",
            input.commit_count
        );
    }

    let log = repo.log_detailed(input.commit_count)?;

    let backend = resolve_backend(backend_config).await?;
    let backend_name = backend.name().to_string();
    let model_name = backend_config
        .model
        .as_deref()
        .unwrap_or("default")
        .to_string();

    let system_prompt = prompts::rebase::system_prompt(input.commit_pattern, input.type_names);
    let user_prompt = prompts::rebase::user_prompt(&log, input.message);

    let request = AiRequest {
        system_prompt,
        user_prompt,
        json_schema: Some(prompts::rebase::SCHEMA.to_string()),
        working_dir: repo.root().to_string_lossy().to_string(),
    };

    let response = backend.request(&request, event_tx).await?;

    let plan: ReorganizePlan = serde_json::from_str(&response.text)
        .or_else(|_| {
            let value: serde_json::Value = serde_json::from_str(&response.text)?;
            serde_json::from_value(value)
        })
        .context("failed to parse rebase plan from AI response")?;

    if plan.commits.is_empty() {
        bail!("AI returned an empty rebase plan");
    }

    let metrics = RebasePlanMetrics {
        backend_name,
        model_name,
        usage: response.usage,
    };

    Ok((plan, metrics))
}

// ---------------------------------------------------------------------------
// Execution
// ---------------------------------------------------------------------------

/// Execute a rebase plan via git rebase -i with generated editor scripts.
pub fn execute_rebase(repo: &GitRepo, plan: &ReorganizePlan, commit_count: usize) -> Result<()> {
    // Build the rebase todo script
    let mut todo_lines = Vec::new();
    for commit in &plan.commits {
        let action = match commit.action.as_str() {
            "pick" | "reword" => "pick",
            "squash" => "squash",
            "drop" => "drop",
            other => bail!("unknown rebase action: {other}"),
        };
        todo_lines.push(format!("{action} {}", commit.original_sha));
    }
    let todo_content = todo_lines.join("\n") + "\n";

    // Build commit message rewrites
    let mut rewrites: HashMap<String, String> = HashMap::new();
    let mut squash_messages: Vec<String> = Vec::new();
    let mut last_pick_sha: Option<String> = None;

    for commit in &plan.commits {
        let mut full_msg = commit.message.clone();
        if let Some(body) = &commit.body {
            if !body.is_empty() {
                full_msg.push_str("\n\n");
                full_msg.push_str(body);
            }
        }
        if let Some(footer) = &commit.footer {
            if !footer.is_empty() {
                full_msg.push_str("\n\n");
                full_msg.push_str(footer);
            }
        }

        match commit.action.as_str() {
            "pick" | "reword" => {
                if !squash_messages.is_empty() {
                    if let Some(ref sha) = last_pick_sha {
                        if let Some(existing) = rewrites.get_mut(sha) {
                            for sq_msg in &squash_messages {
                                existing.push_str("\n\n");
                                existing.push_str(sq_msg);
                            }
                        }
                    }
                    squash_messages.clear();
                }
                last_pick_sha = Some(commit.original_sha.clone());
                rewrites.insert(commit.original_sha.clone(), full_msg);
            }
            "squash" => {
                squash_messages.push(full_msg);
            }
            _ => {}
        }
    }
    // Flush remaining squash messages
    if !squash_messages.is_empty() {
        if let Some(ref sha) = last_pick_sha {
            if let Some(existing) = rewrites.get_mut(sha) {
                for sq_msg in &squash_messages {
                    existing.push_str("\n\n");
                    existing.push_str(sq_msg);
                }
            }
        }
    }

    // Create temp directory for editor scripts
    let tmp_dir = std::env::temp_dir().join(format!("sr-rebase-{}", std::process::id()));
    std::fs::create_dir_all(&tmp_dir).context("failed to create temp dir")?;
    let _cleanup = TmpDirGuard(tmp_dir.clone());

    // Write sequence editor script
    let todo_script_path = tmp_dir.join("sequence-editor.sh");
    {
        let todo_file_path = tmp_dir.join("todo.txt");
        std::fs::write(&todo_file_path, &todo_content)?;
        let script = format!("#!/bin/sh\ncp '{}' \"$1\"\n", todo_file_path.display());
        std::fs::write(&todo_script_path, &script)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&todo_script_path, std::fs::Permissions::from_mode(0o755))?;
        }
    }

    // Write commit message editor script
    let editor_script_path = tmp_dir.join("commit-editor.sh");
    {
        let msgs_dir = tmp_dir.join("msgs");
        std::fs::create_dir_all(&msgs_dir)?;
        for (sha, msg) in &rewrites {
            std::fs::write(msgs_dir.join(sha), msg)?;
        }

        let script = format!(
            r#"#!/bin/sh
MSGS_DIR='{msgs_dir}'
MSG_FILE="$1"

for sha_file in "$MSGS_DIR"/*; do
    sha=$(basename "$sha_file")
    if grep -q "$sha" "$MSG_FILE" 2>/dev/null; then
        cp "$sha_file" "$MSG_FILE"
        exit 0
    fi
done

exit 0
"#,
            msgs_dir = msgs_dir.display()
        );
        std::fs::write(&editor_script_path, &script)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&editor_script_path, std::fs::Permissions::from_mode(0o755))?;
        }
    }

    // Run git rebase
    let base = format!("HEAD~{commit_count}");

    let output = std::process::Command::new("git")
        .args(["-C", repo.root().to_str().unwrap()])
        .args(["rebase", "-i", &base])
        .env("GIT_SEQUENCE_EDITOR", todo_script_path.to_str().unwrap())
        .env("GIT_EDITOR", editor_script_path.to_str().unwrap())
        .env("EDITOR", editor_script_path.to_str().unwrap())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .context("failed to run git rebase")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let _ = std::process::Command::new("git")
            .args(["-C", repo.root().to_str().unwrap()])
            .args(["rebase", "--abort"])
            .output();
        bail!("git rebase failed: {}", stderr.trim());
    }

    Ok(())
}

/// Guard that removes a temp directory on drop.
struct TmpDirGuard(std::path::PathBuf);

impl Drop for TmpDirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
