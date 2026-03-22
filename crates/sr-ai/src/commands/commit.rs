use crate::ai::{AiEvent, AiRequest, BackendConfig, resolve_backend};
use crate::cache::{CacheLookup, CacheManager};
use crate::git::{GitRepo, SnapshotGuard};
use crate::ui;
use anyhow::{Context, Result, bail};
use indicatif::ProgressBar;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;

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

#[derive(Debug, clap::Args)]
pub struct CommitArgs {
    /// Only analyze staged changes
    #[arg(short, long)]
    pub staged: bool,

    /// Additional context or instructions for commit generation
    #[arg(short = 'M', long)]
    pub message: Option<String>,

    /// Display plan without executing
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Skip confirmation prompt
    #[arg(short, long)]
    pub yes: bool,

    /// Bypass cache (always call AI)
    #[arg(long)]
    pub no_cache: bool,

    /// Reorganize recent commits (rename, reorder, squash) using AI
    #[arg(short = 'R', long)]
    pub reorganize: bool,

    /// Number of recent commits to reorganize (default: auto-detect since last tag)
    #[arg(long, requires = "reorganize")]
    pub last: Option<usize>,
}

const COMMIT_SCHEMA: &str = r#"{
    "type": "object",
    "properties": {
        "commits": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "order": { "type": "integer" },
                    "message": { "type": "string", "description": "Header: type(scope): subject — imperative, lowercase, no period, max 72 chars" },
                    "body": { "type": "string", "description": "Body: explain WHY the change was made, wrap at 72 chars" },
                    "footer": { "type": "string", "description": "Footer: BREAKING CHANGE notes, Closes/Fixes/Refs #issue, etc." },
                    "files": { "type": "array", "items": { "type": "string" } }
                },
                "required": ["order", "message", "body", "files"]
            }
        }
    },
    "required": ["commits"]
}"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReorganizePlan {
    pub commits: Vec<ReorganizedCommit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReorganizedCommit {
    /// Original SHA (short) — use "squash" to fold into the previous commit
    pub original_sha: String,
    /// Action: "pick", "reword", "squash", "drop"
    pub action: String,
    /// New commit message (required for pick/reword/squash)
    pub message: String,
    pub body: Option<String>,
    pub footer: Option<String>,
}

const REORGANIZE_SCHEMA: &str = r#"{
    "type": "object",
    "properties": {
        "commits": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "original_sha": { "type": "string", "description": "Short SHA of the original commit" },
                    "action": { "type": "string", "enum": ["pick", "reword", "squash", "drop"], "description": "Rebase action" },
                    "message": { "type": "string", "description": "New commit message header (type(scope): subject)" },
                    "body": { "type": "string", "description": "New commit body (optional)" },
                    "footer": { "type": "string", "description": "New commit footer (optional)" }
                },
                "required": ["original_sha", "action", "message"]
            }
        }
    },
    "required": ["commits"]
}"#;

fn build_system_prompt(commit_pattern: &str, type_names: &[&str]) -> String {
    let types_list = type_names.join(", ");
    format!(
        r#"You are an expert at analyzing git diffs and creating atomic, well-organized commits following the Angular Conventional Commits standard.

HEADER ("message" field):
- Must match this regex: {commit_pattern}
- Format: type(scope): subject
- Valid types ONLY: {types_list}
- NEVER invent types. Words like db, auth, api, etc. are scopes, not types. Use the semantically correct type for the change (e.g. feat(db): add user cache migration, fix(auth): resolve token expiry)
- scope is optional but recommended when applicable
- subject: imperative mood, lowercase first letter, no period at end, max 72 chars

BODY ("body" field — required):
- Explain WHY the change was made, not what changed (the diff shows that)
- Use imperative tense ("add" not "added")
- Wrap at 72 characters

FOOTER ("footer" field — optional):
- BREAKING CHANGE: description of what breaks and migration path
- Closes #N, Fixes #N, Refs #N for issue references
- Only include when relevant

COMMIT ORGANIZATION:
- Each commit must be atomic: one logical change per commit
- Every changed file must appear in exactly one commit
- CRITICAL: A file must NEVER appear in more than one commit. The execution engine stages entire files, not individual hunks. Splitting one file across commits will fail.
- If one file contains multiple logical changes, place it in the most fitting commit and note the secondary changes in that commit's body.
- Order: infrastructure/config -> core library -> features -> tests -> docs
- File paths must be relative to the repository root and match exactly as git reports them"#
    )
}

/// Guard that removes a temp directory on drop.
struct TmpDirGuard(std::path::PathBuf);

impl Drop for TmpDirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

enum CacheStatus {
    /// No cache used (--no-cache, or cache unavailable)
    None,
    /// Exact cache hit
    Cached,
    /// Incremental hit
    Incremental,
}

pub async fn run(args: &CommitArgs, backend_config: &BackendConfig) -> Result<()> {
    if args.reorganize {
        return run_reorganize(args, backend_config).await;
    }

    ui::header("sr commit");

    // Phase 1: Discover repository
    let repo = GitRepo::discover()?;
    ui::phase_ok("Repository found", None);

    // Load project config for commit types and pattern
    let config = sr_core::config::ReleaseConfig::find_config(repo.root().as_path())
        .map(|(path, _)| sr_core::config::ReleaseConfig::load(&path))
        .transpose()?
        .unwrap_or_default();
    let type_names: Vec<&str> = config.types.iter().map(|t| t.name.as_str()).collect();
    let system_prompt = build_system_prompt(&config.commit_pattern, &type_names);

    // Phase 2: Check for changes
    let has_changes = if args.staged {
        repo.has_staged_changes()?
    } else {
        repo.has_any_changes()?
    };

    if !has_changes {
        bail!(crate::error::SrAiError::NoChanges);
    }

    let statuses = repo.file_statuses().unwrap_or_default();
    let file_count = statuses.len();
    ui::phase_ok(
        "Changes detected",
        Some(&format!(
            "{file_count} file{}",
            if file_count == 1 { "" } else { "s" }
        )),
    );

    // Phase 3: Resolve AI backend
    let backend = resolve_backend(backend_config).await?;
    let backend_name = backend.name().to_string();
    let model_name = backend_config
        .model
        .as_deref()
        .unwrap_or("default")
        .to_string();
    ui::phase_ok(
        "Backend resolved",
        Some(&format!("{backend_name} ({model_name})")),
    );

    // Build cache manager (may be None if cache dir unavailable)
    let cache = if args.no_cache {
        None
    } else {
        CacheManager::new(
            repo.root(),
            args.staged,
            args.message.as_deref(),
            &backend_name,
            &model_name,
        )
    };

    // Snapshot the working tree before the agent runs.
    // If anything goes wrong (agent failure, unexpected mutations),
    // the guard restores the working tree from the snapshot on drop.
    let snapshot = SnapshotGuard::new(&repo)?;
    ui::phase_ok("Working tree snapshot saved", None);

    // Phase 4: Generate plan (cache or AI)
    let (mut plan, cache_status) = match cache.as_ref().map(|c| c.lookup()) {
        Some(CacheLookup::ExactHit(cached_plan)) => {
            ui::phase_ok(
                "Plan loaded",
                Some(&format!("{} commits · cached", cached_plan.commits.len())),
            );
            (cached_plan, CacheStatus::Cached)
        }
        Some(CacheLookup::IncrementalHit {
            previous_plan,
            delta_summary,
        }) => {
            let spinner = ui::spinner(&format!(
                "Analyzing changes with {backend_name} (incremental)..."
            ));
            let (tx, event_handler) = spawn_event_handler(&spinner);

            let user_prompt =
                build_incremental_prompt(args, &repo, &previous_plan, &delta_summary)?;

            let request = AiRequest {
                system_prompt: system_prompt.clone(),
                user_prompt,
                json_schema: Some(COMMIT_SCHEMA.to_string()),
                working_dir: repo.root().to_string_lossy().to_string(),
            };

            let response = backend.request(&request, Some(tx)).await?;
            let _ = event_handler.await;

            let p: CommitPlan = parse_plan(&response.text)?;

            let detail = format_done_detail(p.commits.len(), "incremental", &response.usage);
            ui::spinner_done(&spinner, Some(&detail));

            (p, CacheStatus::Incremental)
        }
        _ => {
            let spinner = ui::spinner(&format!("Analyzing changes with {backend_name}..."));
            let (tx, event_handler) = spawn_event_handler(&spinner);

            let user_prompt = build_user_prompt(args, &repo)?;

            let request = AiRequest {
                system_prompt: system_prompt.clone(),
                user_prompt,
                json_schema: Some(COMMIT_SCHEMA.to_string()),
                working_dir: repo.root().to_string_lossy().to_string(),
            };

            let response = backend.request(&request, Some(tx)).await?;
            let _ = event_handler.await;

            let p: CommitPlan = parse_plan(&response.text)?;

            let detail = format_done_detail(p.commits.len(), "", &response.usage);
            ui::spinner_done(&spinner, Some(&detail));

            (p, CacheStatus::None)
        }
    };

    if plan.commits.is_empty() {
        bail!(crate::error::SrAiError::EmptyPlan);
    }

    // Validate: merge commits with shared files
    let pre_validate_count = plan.commits.len();
    plan = validate_plan(plan);
    if plan.commits.len() < pre_validate_count {
        ui::warn(&format!(
            "Shared files detected — merged {} commits into 1",
            pre_validate_count - plan.commits.len() + 1
        ));
    }

    // Store in cache (before display/execute so dry-runs populate cache too)
    if let Some(cache) = &cache {
        cache.store(&plan, &backend_name, &model_name);
    }

    // Display plan
    let cache_label: Option<&str> = match &cache_status {
        CacheStatus::Cached => Some("cached"),
        CacheStatus::Incremental => Some("incremental"),
        CacheStatus::None => None,
    };
    ui::display_plan(&plan, &statuses, cache_label);

    if args.dry_run {
        ui::info("Dry run — no commits created");
        println!();
        return Ok(());
    }

    // Confirm
    if !args.yes && !ui::confirm("Execute plan? [y/N]")? {
        bail!(crate::error::SrAiError::Cancelled);
    }

    // Execute
    execute_plan(&repo, &plan)?;

    // All commits succeeded — clear the snapshot
    snapshot.success();

    Ok(())
}

async fn run_reorganize(args: &CommitArgs, backend_config: &BackendConfig) -> Result<()> {
    ui::header("sr commit --reorganize");

    let repo = GitRepo::discover()?;
    ui::phase_ok("Repository found", None);

    if repo.has_any_changes()? {
        bail!(
            "cannot reorganize: you have uncommitted changes. Please commit or stash them first."
        );
    }

    // Load config for commit pattern/types
    let config = sr_core::config::ReleaseConfig::find_config(repo.root().as_path())
        .map(|(path, _)| sr_core::config::ReleaseConfig::load(&path))
        .transpose()?
        .unwrap_or_default();
    let type_names: Vec<&str> = config.types.iter().map(|t| t.name.as_str()).collect();

    // Determine how many commits to reorganize
    let commit_count = match args.last {
        Some(n) => n,
        None => {
            // Auto-detect: count commits since last tag
            let count = repo.commits_since_last_tag()?;
            if count == 0 {
                bail!("no commits found to reorganize");
            }
            count
        }
    };

    if commit_count < 2 {
        bail!("need at least 2 commits to reorganize (found {commit_count})");
    }

    // Get commit details
    let log = repo.log_detailed(commit_count)?;
    ui::phase_ok("Commits loaded", Some(&format!("{commit_count} commits")));

    // Resolve AI backend
    let backend = resolve_backend(backend_config).await?;
    let backend_name = backend.name().to_string();
    let model_name = backend_config
        .model
        .as_deref()
        .unwrap_or("default")
        .to_string();
    ui::phase_ok(
        "Backend resolved",
        Some(&format!("{backend_name} ({model_name})")),
    );

    // Build prompt
    let system_prompt = build_reorganize_system_prompt(&config.commit_pattern, &type_names);
    let user_prompt = build_reorganize_user_prompt(&log, args.message.as_deref())?;

    let spinner = ui::spinner(&format!("Analyzing commits with {backend_name}..."));
    let (tx, event_handler) = spawn_event_handler(&spinner);

    let request = AiRequest {
        system_prompt,
        user_prompt,
        json_schema: Some(REORGANIZE_SCHEMA.to_string()),
        working_dir: repo.root().to_string_lossy().to_string(),
    };

    let response = backend.request(&request, Some(tx)).await?;
    let _ = event_handler.await;

    let plan: ReorganizePlan = serde_json::from_str(&response.text)
        .or_else(|_| {
            let value: serde_json::Value = serde_json::from_str(&response.text)?;
            serde_json::from_value(value)
        })
        .context("failed to parse reorganize plan from AI response")?;

    let detail = format_done_detail(plan.commits.len(), "", &response.usage);
    ui::spinner_done(&spinner, Some(&detail));

    if plan.commits.is_empty() {
        bail!("AI returned an empty reorganization plan");
    }

    // Display the plan
    display_reorganize_plan(&plan);

    if args.dry_run {
        ui::info("Dry run — no changes made");
        println!();
        return Ok(());
    }

    if !args.yes && !ui::confirm("Execute reorganization? [y/N]")? {
        bail!(crate::error::SrAiError::Cancelled);
    }

    // Execute via git rebase
    execute_reorganize(&repo, &plan, commit_count)?;

    Ok(())
}

fn build_reorganize_system_prompt(commit_pattern: &str, type_names: &[&str]) -> String {
    let types_list = type_names.join(", ");
    format!(
        r#"You are an expert at organizing git history. You will be given a list of recent commits and asked to reorganize them.

You can:
- **pick**: keep the commit as-is (but you may reword the message)
- **reword**: keep the commit but change the message
- **squash**: fold the commit into the previous one (combine their changes)
- **drop**: remove the commit entirely (use sparingly — only for truly empty or duplicate commits)

COMMIT MESSAGE FORMAT:
- Must match this regex: {commit_pattern}
- Format: type(scope): subject
- Valid types ONLY: {types_list}
- subject: imperative mood, lowercase first letter, no period at end, max 72 chars

RULES:
- Maintain the chronological order of commits (oldest first) unless reordering improves logical grouping
- The first commit in the list CANNOT be "squash" — squash folds into the previous commit
- Prefer "reword" over "squash" when commits are logically distinct
- Only squash commits that are genuinely part of the same logical change
- Every original commit SHA must appear exactly once in your output
- If the commits are already well-organized, return them all as "pick" with improved messages if needed"#
    )
}

fn build_reorganize_user_prompt(log: &str, extra: Option<&str>) -> Result<String> {
    let mut prompt = format!(
        "Analyze these recent commits and suggest how to reorganize them for a cleaner history.\n\n\
         Commits (oldest first):\n```\n{log}\n```"
    );

    if let Some(msg) = extra {
        prompt.push_str(&format!(
            "\n\nAdditional instructions from the user:\n{msg}"
        ));
    }

    Ok(prompt)
}

fn display_reorganize_plan(plan: &ReorganizePlan) {
    use crossterm::style::Stylize;

    println!();
    println!(
        "  {} {}",
        "REORGANIZE PLAN".bold(),
        format!("· {} commits", plan.commits.len()).dim()
    );
    let rule = "─".repeat(50);
    println!("  {}", rule.as_str().dim());
    println!();

    for commit in &plan.commits {
        let action_styled = match commit.action.as_str() {
            "pick" => format!("{}", "pick".green()),
            "reword" => format!("{}", "reword".yellow()),
            "squash" => format!("{}", "squash".magenta()),
            "drop" => format!("{}", "drop".red()),
            other => other.to_string(),
        };

        println!(
            "  {} {} {}",
            action_styled,
            commit.original_sha.as_str().dim(),
            commit.message.as_str().bold()
        );

        if let Some(body) = &commit.body
            && !body.is_empty()
        {
            for line in body.lines() {
                println!("   {}  {}", "│".dim(), line.dim());
            }
        }
    }

    println!();
    println!("  {}", rule.as_str().dim());
    println!();
}

fn execute_reorganize(repo: &GitRepo, plan: &ReorganizePlan, commit_count: usize) -> Result<()> {
    // Build the rebase todo script
    let mut todo_lines = Vec::new();
    for commit in &plan.commits {
        let action = match commit.action.as_str() {
            "pick" | "reword" => "pick", // we'll force-reword via GIT_SEQUENCE_EDITOR
            "squash" => "squash",
            "drop" => "drop",
            other => bail!("unknown rebase action: {other}"),
        };
        todo_lines.push(format!("{action} {}", commit.original_sha));
    }
    let todo_content = todo_lines.join("\n") + "\n";

    // Build commit message rewrites: map SHA -> new full message
    let mut rewrites: HashMap<String, String> = HashMap::new();
    // Also track squash messages to combine
    let mut squash_messages: Vec<String> = Vec::new();
    let mut last_pick_sha: Option<String> = None;

    for commit in &plan.commits {
        let mut full_msg = commit.message.clone();
        if let Some(body) = &commit.body
            && !body.is_empty()
        {
            full_msg.push_str("\n\n");
            full_msg.push_str(body);
        }
        if let Some(footer) = &commit.footer
            && !footer.is_empty()
        {
            full_msg.push_str("\n\n");
            full_msg.push_str(footer);
        }

        match commit.action.as_str() {
            "pick" | "reword" => {
                // Flush any pending squash messages into the last pick
                if !squash_messages.is_empty() {
                    if let Some(ref sha) = last_pick_sha
                        && let Some(existing) = rewrites.get_mut(sha)
                    {
                        for sq_msg in &squash_messages {
                            existing.push_str("\n\n");
                            existing.push_str(sq_msg);
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
    if !squash_messages.is_empty()
        && let Some(ref sha) = last_pick_sha
        && let Some(existing) = rewrites.get_mut(sha)
    {
        for sq_msg in &squash_messages {
            existing.push_str("\n\n");
            existing.push_str(sq_msg);
        }
    }

    // Create a temporary directory for our editor scripts
    let tmp_dir = std::env::temp_dir().join(format!("sr-reorganize-{}", std::process::id()));
    std::fs::create_dir_all(&tmp_dir).context("failed to create temp dir")?;
    // Ensure cleanup on exit
    let _cleanup = TmpDirGuard(tmp_dir.clone());

    // Write the todo script (used as GIT_SEQUENCE_EDITOR)
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

    // Write the commit message editor script (used as GIT_EDITOR / EDITOR)
    let editor_script_path = tmp_dir.join("commit-editor.sh");
    {
        // Write each rewrite message to a file named by SHA
        let msgs_dir = tmp_dir.join("msgs");
        std::fs::create_dir_all(&msgs_dir)?;
        for (sha, msg) in &rewrites {
            std::fs::write(msgs_dir.join(sha), msg)?;
        }

        // The editor script: given a commit message file, find the matching SHA
        // and replace with our rewritten message. For squash commits, git presents
        // a combined message — we replace it entirely with the pick commit's message.
        let script = format!(
            r#"#!/bin/sh
MSGS_DIR='{msgs_dir}'
MSG_FILE="$1"

# Try to find a matching SHA in the message file
for sha_file in "$MSGS_DIR"/*; do
    sha=$(basename "$sha_file")
    if grep -q "$sha" "$MSG_FILE" 2>/dev/null; then
        cp "$sha_file" "$MSG_FILE"
        exit 0
    fi
done

# For squash: the combined message won't contain a single SHA.
# Find the first pick/reword SHA that's referenced in the todo.
# Just use the message as-is if we can't match.
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

    // Run git rebase -i with our custom editors
    let base = format!("HEAD~{commit_count}");

    ui::info(&format!("Rebasing {commit_count} commits..."));

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
        // Abort the rebase if it failed
        let _ = std::process::Command::new("git")
            .args(["-C", repo.root().to_str().unwrap()])
            .args(["rebase", "--abort"])
            .output();
        bail!("git rebase failed: {}", stderr.trim());
    }

    // Show the new history
    let new_log = repo.recent_commits(commit_count)?;
    println!();
    ui::phase_ok("Reorganization complete", None);
    println!();
    for line in new_log.lines() {
        println!("    {line}");
    }
    println!();

    Ok(())
}

fn build_user_prompt(args: &CommitArgs, repo: &GitRepo) -> Result<String> {
    let git_root = repo.root().to_string_lossy();

    let mut prompt = if args.staged {
        "Analyze the staged git changes and group them into atomic commits.\n\
         Use `git diff --cached` and `git diff --cached --stat` to inspect what's staged."
            .to_string()
    } else {
        "Analyze all git changes (staged, unstaged, and untracked) and group them into atomic commits.\n\
         Use `git diff HEAD`, `git diff --cached`, `git diff`, `git status --porcelain`, and \
         `git ls-files --others --exclude-standard` to inspect changes."
            .to_string()
    };

    prompt.push_str(&format!("\nThe git repository root is: {git_root}"));

    if let Some(msg) = &args.message {
        prompt.push_str(&format!("\n\nAdditional context from the user:\n{msg}"));
    }

    Ok(prompt)
}

fn build_incremental_prompt(
    args: &CommitArgs,
    repo: &GitRepo,
    previous_plan: &CommitPlan,
    delta_summary: &str,
) -> Result<String> {
    let mut prompt = build_user_prompt(args, repo)?;

    let previous_json =
        serde_json::to_string_pretty(previous_plan).unwrap_or_else(|_| "{}".to_string());

    prompt.push_str(&format!(
        "\n\n--- INCREMENTAL HINTS ---\n\
         A previous commit plan exists for a similar set of changes. \
         Maintain the groupings for unchanged files where possible. \
         Only re-analyze files that have changed.\n\n\
         Previous plan:\n```json\n{previous_json}\n```\n\n\
         File delta:\n{delta_summary}"
    ));

    Ok(prompt)
}

/// Validate that no file appears in multiple commits. If duplicates are found,
/// merge affected commits into one.
fn validate_plan(plan: CommitPlan) -> CommitPlan {
    // Count file occurrences
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

    // Partition into tainted (has any dupe file) and clean
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

    // Merge all tainted commits into one
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

    // Re-number: merged first, then clean commits
    let mut result = vec![merged_commit];
    for (i, mut commit) in clean.into_iter().enumerate() {
        commit.order = Some(i as u32 + 2);
        result.push(commit);
    }

    CommitPlan { commits: result }
}

/// Parse a commit plan from JSON text, tolerating duplicate fields.
fn parse_plan(text: &str) -> Result<CommitPlan> {
    // Parse to Value first — serde_json::Value keeps the last value for duplicate keys,
    // while #[derive(Deserialize)] rejects them. This handles AI responses that
    // occasionally produce duplicate fields when schema is embedded in the prompt.
    let value: serde_json::Value =
        serde_json::from_str(text).context("failed to parse JSON from AI response")?;
    serde_json::from_value(value).context("failed to parse commit plan from AI response")
}

/// Spawn a background task that renders AI events (tool calls) above a spinner.
fn spawn_event_handler(
    spinner: &ProgressBar,
) -> (mpsc::UnboundedSender<AiEvent>, tokio::task::JoinHandle<()>) {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let pb = spinner.clone();
    let handle = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                AiEvent::ToolCall { input, .. } => ui::tool_call(&pb, &input),
            }
        }
    });
    (tx, handle)
}

/// Format the detail string for spinner_done, including usage if available.
fn format_done_detail(
    commit_count: usize,
    extra: &str,
    usage: &Option<crate::ai::AiUsage>,
) -> String {
    let commits = format!(
        "{commit_count} commit{}",
        if commit_count == 1 { "" } else { "s" }
    );
    let extra_part = if extra.is_empty() {
        String::new()
    } else {
        format!(" · {extra}")
    };
    let usage_part = match usage {
        Some(u) => {
            let cost = u
                .cost_usd
                .map(|c| format!(" · ${c:.4}"))
                .unwrap_or_default();
            format!(
                " · {} in / {} out{}",
                ui::format_tokens(u.input_tokens),
                ui::format_tokens(u.output_tokens),
                cost
            )
        }
        None => String::new(),
    };
    format!("{commits}{extra_part}{usage_part}")
}

fn execute_plan(repo: &GitRepo, plan: &CommitPlan) -> Result<()> {
    // Unstage everything first
    repo.reset_head()?;

    let total = plan.commits.len();
    let mut created: Vec<(String, String)> = Vec::new();

    for (i, commit) in plan.commits.iter().enumerate() {
        ui::commit_start(i + 1, total, &commit.message);

        // Stage files for this commit
        for file in &commit.files {
            let ok = repo.stage_file(file)?;
            ui::file_staged(file, ok);
        }

        // Build full commit message
        let mut full_message = commit.message.clone();
        if let Some(body) = &commit.body
            && !body.is_empty()
        {
            full_message.push_str("\n\n");
            full_message.push_str(body);
        }
        if let Some(footer) = &commit.footer
            && !footer.is_empty()
        {
            full_message.push_str("\n\n");
            full_message.push_str(footer);
        }

        // Create commit (only if there are staged files)
        if repo.has_staged_after_add()? {
            repo.commit(&full_message)?;
            let sha = repo.head_short().unwrap_or_else(|_| "???????".to_string());
            ui::commit_created(&sha);
            created.push((sha, commit.message.clone()));
        } else {
            ui::commit_skipped();
        }
    }

    ui::summary(&created);

    Ok(())
}

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
        // Two tainted merged into one + one clean = 2
        assert_eq!(result.commits.len(), 2);
        assert_eq!(result.commits[0].message, "feat: add foo");
        assert!(result.commits[0].files.contains(&"shared.rs".to_string()));
        assert!(result.commits[0].files.contains(&"a.rs".to_string()));
        assert!(result.commits[0].files.contains(&"b.rs".to_string()));
        assert_eq!(result.commits[1].message, "docs: update readme");
        assert_eq!(result.commits[1].order, Some(2));
    }
}
