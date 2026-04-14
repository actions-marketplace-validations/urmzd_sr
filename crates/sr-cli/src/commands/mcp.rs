use anyhow::Result;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::schemars::{self, JsonSchema};
use rmcp::{ServiceExt, tool, tool_router};
use serde::{Deserialize, Serialize};
use sr_core::git::GitRepo;

/// MCP server exposing sr's git operations as tools.
///
/// AI clients (Claude Code, Gemini CLI, etc.) connect to this server
/// and use the tools to inspect and modify the repository.
#[derive(Debug, Clone)]
pub struct SrMcpServer;

// --- Tool parameter types (schemas enforce correct input, prevent hallucination) ---

#[derive(Deserialize, JsonSchema)]
pub struct DiffParams {
    /// Only show staged changes (git diff --cached)
    #[serde(default)]
    pub staged: bool,
    /// Specific files to diff (empty = all changed files)
    #[serde(default)]
    pub files: Vec<String>,
    /// Number of unchanged context lines around each change (default: 0).
    /// Use 0 for minimal output (just the changed lines), increase for more surrounding code.
    #[serde(default)]
    pub context: usize,
    /// Return only the file list with stats, no line-level diffs.
    /// Use this first to see what changed, then request specific files.
    #[serde(default)]
    pub name_only: bool,
}

// --- Structured diff output types ---

#[derive(Serialize)]
struct DiffOutput {
    files: Vec<FileDiff>,
    total_additions: usize,
    total_deletions: usize,
}

#[derive(Serialize)]
struct FileDiff {
    path: String,
    status: char,
    additions: usize,
    deletions: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    hunks: Vec<Hunk>,
}

#[derive(Serialize)]
struct Hunk {
    /// Starting line in the old file
    old_start: usize,
    /// Number of lines in the old file
    old_lines: usize,
    /// Starting line in the new file
    new_start: usize,
    /// Number of lines in the new file
    new_lines: usize,
    /// Individual line changes
    changes: Vec<Change>,
}

#[derive(Serialize)]
struct Change {
    /// "add", "delete", or "context"
    kind: &'static str,
    /// Line number in the relevant file (new for add/context, old for delete)
    line: usize,
    /// The line content (without the +/- prefix)
    content: String,
}

fn parse_unified_diff(raw: &str) -> Vec<(String, Vec<Hunk>)> {
    let mut files: Vec<(String, Vec<Hunk>)> = Vec::new();
    let mut current_path: Option<String> = None;
    let mut hunks: Vec<Hunk> = Vec::new();
    let mut changes: Vec<Change> = Vec::new();
    let mut hunk_header: Option<(usize, usize, usize, usize)> = None;
    let mut old_cursor: usize = 0;
    let mut new_cursor: usize = 0;

    for line in raw.lines() {
        if line.starts_with("diff --git ") {
            // Flush previous hunk
            if let Some((os, ol, ns, nl)) = hunk_header.take() {
                hunks.push(Hunk {
                    old_start: os,
                    old_lines: ol,
                    new_start: ns,
                    new_lines: nl,
                    changes: std::mem::take(&mut changes),
                });
            }
            // Flush previous file
            if let Some(path) = current_path.take() {
                files.push((path, std::mem::take(&mut hunks)));
            }
            if let Some(b_part) = line.split(" b/").last() {
                current_path = Some(b_part.to_string());
            }
            continue;
        }

        if line.starts_with("@@ ") {
            // Flush previous hunk
            if let Some((os, ol, ns, nl)) = hunk_header.take() {
                hunks.push(Hunk {
                    old_start: os,
                    old_lines: ol,
                    new_start: ns,
                    new_lines: nl,
                    changes: std::mem::take(&mut changes),
                });
            }
            if let Some(header) = line.strip_prefix("@@ ") {
                let parts: Vec<&str> = header.splitn(3, ' ').collect();
                if parts.len() >= 2 {
                    let (os, ol) = parse_hunk_range(parts[0].trim_start_matches('-'));
                    let (ns, nl) = parse_hunk_range(parts[1].trim_start_matches('+'));
                    old_cursor = os;
                    new_cursor = ns;
                    hunk_header = Some((os, ol, ns, nl));
                }
            }
            continue;
        }

        if line.starts_with("index ")
            || line.starts_with("--- ")
            || line.starts_with("+++ ")
            || line.starts_with("old mode")
            || line.starts_with("new mode")
            || line.starts_with("new file")
            || line.starts_with("deleted file")
            || line.starts_with("similarity")
            || line.starts_with("rename ")
            || line.starts_with("Binary ")
        {
            continue;
        }

        if hunk_header.is_some() {
            if let Some(content) = line.strip_prefix('+') {
                changes.push(Change {
                    kind: "add",
                    line: new_cursor,
                    content: content.to_string(),
                });
                new_cursor += 1;
            } else if let Some(content) = line.strip_prefix('-') {
                changes.push(Change {
                    kind: "delete",
                    line: old_cursor,
                    content: content.to_string(),
                });
                old_cursor += 1;
            } else if let Some(content) = line.strip_prefix(' ') {
                changes.push(Change {
                    kind: "context",
                    line: new_cursor,
                    content: content.to_string(),
                });
                old_cursor += 1;
                new_cursor += 1;
            }
        }
    }

    // Flush final hunk and file
    if let Some((os, ol, ns, nl)) = hunk_header {
        hunks.push(Hunk {
            old_start: os,
            old_lines: ol,
            new_start: ns,
            new_lines: nl,
            changes,
        });
    }
    if let Some(path) = current_path {
        files.push((path, hunks));
    }

    files
}

fn parse_hunk_range(s: &str) -> (usize, usize) {
    if let Some((start, count)) = s.split_once(',') {
        (start.parse().unwrap_or(0), count.parse().unwrap_or(0))
    } else {
        (s.parse().unwrap_or(0), 1)
    }
}

#[derive(Deserialize, JsonSchema)]
pub struct LogParams {
    /// Number of recent commits to show (default: 10)
    #[serde(default = "default_log_count")]
    pub count: usize,
    /// Git range expression (e.g. "main..HEAD"). Overrides count.
    pub range: Option<String>,
}

fn default_log_count() -> usize {
    10
}

#[derive(Deserialize, JsonSchema)]
pub struct StageParams {
    /// Files to stage. Use ["."] to stage all changes.
    pub files: Vec<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct CommitParams {
    /// Commit type (feat, fix, chore, docs, style, refactor, test, ci, build, perf)
    pub r#type: String,
    /// Scope of the change (e.g. "cli", "core", "auth"). Optional.
    pub scope: Option<String>,
    /// Short description of the change (imperative mood, no period)
    pub description: String,
    /// Whether this is a breaking change. Adds "!" after type/scope in the header.
    #[serde(default)]
    pub breaking: bool,
    /// Extended description with motivation and context. Optional.
    /// Separate paragraphs with blank lines.
    pub body: Option<String>,
    /// Conventional commit footer lines. Optional.
    /// Each footer should be "token: value" or "token #value".
    /// Common tokens: BREAKING CHANGE, Closes, Fixes, Refs, Reviewed-by.
    /// Multiple footers can be separated by newlines.
    pub footer: Option<String>,
    /// Specific files to include in this commit. Empty = all staged files.
    #[serde(default)]
    pub files: Vec<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct PrCreateParams {
    /// PR title. Should be concise (under 70 chars).
    pub title: String,
    /// PR body in markdown. Should include a summary section and test plan.
    pub body: String,
    /// Target base branch. Defaults to the repo's default branch.
    pub base: Option<String>,
    /// Create as draft PR.
    #[serde(default)]
    pub draft: bool,
    /// Labels to add to the PR.
    #[serde(default)]
    pub labels: Vec<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct PrTemplateParams {
    /// Optional: override the base branch to diff against (default: repo default branch).
    pub base: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct WorktreeParams {
    /// Branch name for the worktree (e.g. "feat/add-auth").
    pub branch: String,
    /// Short description of the worktree's purpose (e.g. "OAuth2 integration").
    /// Stored in .sr/worktrees/ metadata for tracking.
    pub description: String,
    /// Base ref to branch from. Defaults to HEAD.
    pub base: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct WorktreeListParams {}

#[derive(Deserialize, JsonSchema)]
pub struct WorktreeRemoveParams {
    /// Branch name of the worktree to remove.
    pub branch: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct BranchParams {
    /// Branch name to create. Must use conventional format (e.g. "feat/add-auth")
    pub name: Option<String>,
}

// --- Tool implementations ---

#[tool_router(server_handler)]
impl SrMcpServer {
    /// Get the current repository status: changed files with their status indicators
    /// (M=modified, A=added, D=deleted, ?=untracked) and SHA-256 fingerprints.
    /// Use this first to understand what changed, then call sr_diff for specific files.
    #[tool(
        name = "sr_status",
        description = "Get repository status with file fingerprints. Call this first to see what changed."
    )]
    async fn status(&self) -> String {
        let repo = match GitRepo::discover() {
            Ok(r) => r,
            Err(e) => return format!("error: {e}"),
        };

        let status = match repo.status_porcelain() {
            Ok(s) => s,
            Err(e) => return format!("error: {e}"),
        };

        let statuses = match repo.file_statuses() {
            Ok(s) => s,
            Err(e) => return format!("error: {e}"),
        };

        let mut result = String::from("# Repository Status\n\n");
        if status.trim().is_empty() {
            result.push_str("No changes.\n");
            return result;
        }

        for line in status.lines() {
            if !line.is_empty() {
                result.push_str(line);
                result.push('\n');
            }
        }

        result.push_str(&format!("\n{} file(s) changed\n", statuses.len()));
        result
    }

    /// Get structured diff data for changed files. Returns JSON with per-file
    /// changes, line-level hunks, and stats. Use `name_only` for a quick summary,
    /// then request specific `files` with the exact changed lines for classification.
    #[tool(
        name = "sr_diff",
        description = "Get structured diff: per-file stats + line-level changes as JSON. Use name_only for file list, then drill into specific files."
    )]
    async fn diff(&self, Parameters(params): Parameters<DiffParams>) -> String {
        let repo = match GitRepo::discover() {
            Ok(r) => r,
            Err(e) => return format!("{{\"error\":\"{e}\"}}"),
        };

        // Get per-file stats
        let stats = match repo.diff_numstat(params.staged, &params.files) {
            Ok(s) => s,
            Err(e) => return format!("{{\"error\":\"{e}\"}}"),
        };

        if stats.is_empty() {
            return "{\"files\":[],\"total_additions\":0,\"total_deletions\":0}".to_string();
        }

        // Get file statuses for the status character
        let statuses = repo.file_statuses().unwrap_or_default();

        // Build per-file hunks (unless name_only)
        let parsed_hunks = if params.name_only {
            Vec::new()
        } else {
            match repo.diff_unified(params.staged, params.context, &params.files) {
                Ok(raw) => parse_unified_diff(&raw),
                Err(_) => Vec::new(),
            }
        };

        // Index hunks by path for lookup
        let hunk_map: std::collections::HashMap<&str, &Vec<Hunk>> =
            parsed_hunks.iter().map(|(p, h)| (p.as_str(), h)).collect();

        let mut total_add = 0;
        let mut total_del = 0;
        let mut file_diffs = Vec::new();

        for (add, del, path) in &stats {
            total_add += add;
            total_del += del;
            let status = statuses.get(path.as_str()).copied().unwrap_or('M');
            let hunks = if params.name_only {
                Vec::new()
            } else if let Some(h) = hunk_map.get(path.as_str()) {
                // Re-serialize the parsed hunks (they're borrowed, need to clone)
                h.iter()
                    .map(|hunk| Hunk {
                        old_start: hunk.old_start,
                        old_lines: hunk.old_lines,
                        new_start: hunk.new_start,
                        new_lines: hunk.new_lines,
                        changes: hunk
                            .changes
                            .iter()
                            .map(|c| Change {
                                kind: c.kind,
                                line: c.line,
                                content: c.content.clone(),
                            })
                            .collect(),
                    })
                    .collect()
            } else {
                Vec::new()
            };
            file_diffs.push(FileDiff {
                path: path.clone(),
                status,
                additions: *add,
                deletions: *del,
                hunks,
            });
        }

        let output = DiffOutput {
            files: file_diffs,
            total_additions: total_add,
            total_deletions: total_del,
        };

        serde_json::to_string(&output).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
    }

    /// Get the commit log. Use `range` for specific ranges (e.g. "main..HEAD" for PR commits)
    /// or `count` for recent N commits.
    #[tool(
        name = "sr_log",
        description = "Get commit log. Use range for PR commits or count for recent history."
    )]
    async fn log(&self, Parameters(params): Parameters<LogParams>) -> String {
        let repo = match GitRepo::discover() {
            Ok(r) => r,
            Err(e) => return format!("error: {e}"),
        };

        if let Some(range) = &params.range {
            match repo.log_range(range, None) {
                Ok(log) => log,
                Err(e) => format!("error: {e}"),
            }
        } else {
            match repo.recent_commits(params.count) {
                Ok(log) => log,
                Err(e) => format!("error: {e}"),
            }
        }
    }

    /// Stage files for commit. Use ["."] to stage all changes.
    /// WARNING: This modifies the repository index.
    #[tool(
        name = "sr_stage",
        description = "Stage files for commit. Use [\".\"] for all changes. Modifies the index."
    )]
    async fn stage(&self, Parameters(params): Parameters<StageParams>) -> String {
        let repo = match GitRepo::discover() {
            Ok(r) => r,
            Err(e) => return format!("error: {e}"),
        };

        if params.files.is_empty() {
            return "error: no files specified".to_string();
        }

        let mut staged = Vec::new();
        let mut failed = Vec::new();

        for file in &params.files {
            if file == "." {
                let s = std::process::Command::new("git")
                    .args(["-C", &repo.root().to_string_lossy()])
                    .args(["add", "-A"])
                    .status();
                match s {
                    Ok(s) if s.success() => staged.push("all files".to_string()),
                    _ => failed.push("all files".to_string()),
                }
            } else {
                match repo.stage_file(file) {
                    Ok(true) => staged.push(file.clone()),
                    _ => failed.push(file.clone()),
                }
            }
        }

        let mut result = String::new();
        if !staged.is_empty() {
            result.push_str(&format!("staged: {}\n", staged.join(", ")));
        }
        if !failed.is_empty() {
            result.push_str(&format!("failed: {}\n", failed.join(", ")));
        }
        result
    }

    /// Create a commit with a conventional commit message.
    /// Format: type(scope): description
    /// The type, scope, description, body, and footer are structured to prevent
    /// malformed commit messages.
    /// WARNING: This creates a git commit. Ensure files are staged first.
    #[tool(
        name = "sr_commit",
        description = "Create a conventional commit. Stage files first with sr_stage."
    )]
    async fn commit(&self, Parameters(params): Parameters<CommitParams>) -> String {
        let repo = match GitRepo::discover() {
            Ok(r) => r,
            Err(e) => return format!("error: {e}"),
        };

        // Stage specific files if provided
        if !params.files.is_empty() {
            for file in &params.files {
                let _ = repo.stage_file(file);
            }
        }

        match repo.has_staged_changes() {
            Ok(false) => return "error: no staged changes to commit".to_string(),
            Err(e) => return format!("error: {e}"),
            _ => {}
        }

        // Build conventional commit message
        let bang = if params.breaking { "!" } else { "" };
        let header = match &params.scope {
            Some(scope) => {
                format!(
                    "{}({}){}: {}",
                    params.r#type, scope, bang, params.description
                )
            }
            None => format!("{}{}: {}", params.r#type, bang, params.description),
        };

        let mut message = header.clone();
        if let Some(body) = &params.body {
            message.push_str("\n\n");
            message.push_str(body);
        }
        // If breaking and no explicit BREAKING CHANGE footer, add one
        if params.breaking {
            let has_breaking_footer = params
                .footer
                .as_deref()
                .is_some_and(|f| f.contains("BREAKING CHANGE"));
            if !has_breaking_footer {
                message.push_str("\n\n");
                message.push_str(&format!("BREAKING CHANGE: {}", params.description));
            }
        }
        if let Some(footer) = &params.footer {
            message.push_str("\n\n");
            message.push_str(footer);
        }

        match repo.commit(&message) {
            Ok(()) => {
                let sha = repo.head_short().unwrap_or_else(|_| "???".to_string());
                format!("{sha}  {header}")
            }
            Err(e) => format!("error: {e}"),
        }
    }

    /// Get or create a branch. Without a name, returns the current branch.
    /// With a name, creates a new branch and switches to it.
    #[tool(
        name = "sr_branch",
        description = "Get current branch or create a new one."
    )]
    async fn branch(&self, Parameters(params): Parameters<BranchParams>) -> String {
        let repo = match GitRepo::discover() {
            Ok(r) => r,
            Err(e) => return format!("error: {e}"),
        };

        match params.name {
            None => match repo.current_branch() {
                Ok(b) => b,
                Err(e) => format!("error: {e}"),
            },
            Some(name) => {
                let status = std::process::Command::new("git")
                    .args(["-C", &repo.root().to_string_lossy()])
                    .args(["checkout", "-b", &name])
                    .status();
                match status {
                    Ok(s) if s.success() => format!("created and switched to branch: {name}"),
                    _ => format!("error: failed to create branch {name}"),
                }
            }
        }
    }

    /// Generate a PR template based on the commits on the current branch.
    /// Returns a structured template with title, body sections, and metadata
    /// that the AI assistant should fill in before calling sr_pr_create.
    #[tool(
        name = "sr_pr_template",
        description = "Generate a PR template from current branch commits. Returns a template to fill in before creating the PR."
    )]
    async fn pr_template(&self, Parameters(params): Parameters<PrTemplateParams>) -> String {
        let repo = match GitRepo::discover() {
            Ok(r) => r,
            Err(e) => return format!("error: {e}"),
        };

        let branch = match repo.current_branch() {
            Ok(b) => b,
            Err(e) => return format!("error: {e}"),
        };

        let base = params.base.as_deref().unwrap_or("main");

        // Get commits on this branch vs base
        let output = std::process::Command::new("git")
            .args(["-C", &repo.root().to_string_lossy()])
            .args(["log", "--oneline", &format!("{base}..HEAD")])
            .output();

        let commits = match output {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
            _ => "(no commits found)".to_string(),
        };

        // Get diff stats
        let stats_output = std::process::Command::new("git")
            .args(["-C", &repo.root().to_string_lossy()])
            .args(["diff", "--stat", &format!("{base}...HEAD")])
            .output();

        let stats = match stats_output {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
            _ => "(no diff stats)".to_string(),
        };

        serde_json::to_string_pretty(&serde_json::json!({
            "branch": branch,
            "base": base,
            "commits": commits,
            "diff_stats": stats,
            "template": {
                "title": "<concise title under 70 chars>",
                "body": "## Summary\n<1-3 bullet points describing what changed and why>\n\n## Test plan\n<how to verify this works>\n",
                "draft": false,
                "labels": []
            },
            "instructions": "Fill in the template fields based on the commits and diff, then call sr_pr_create."
        }))
        .unwrap_or_else(|e| format!("error: {e}"))
    }

    /// Create a pull request on GitHub. Use sr_pr_template first to generate
    /// a template, then fill it in and pass it here.
    /// Requires `gh` CLI to be installed and authenticated.
    #[tool(
        name = "sr_pr_create",
        description = "Create a GitHub PR. Use sr_pr_template first to get a template, fill it in, then call this."
    )]
    async fn pr_create(&self, Parameters(params): Parameters<PrCreateParams>) -> String {
        let repo = match GitRepo::discover() {
            Ok(r) => r,
            Err(e) => return format!("error: {e}"),
        };

        // Push current branch first
        let branch = match repo.current_branch() {
            Ok(b) => b,
            Err(e) => return format!("error: {e}"),
        };

        let push = std::process::Command::new("git")
            .args(["-C", &repo.root().to_string_lossy()])
            .args(["push", "-u", "origin", &branch])
            .output();

        if let Ok(o) = &push
            && !o.status.success()
        {
            let err = String::from_utf8_lossy(&o.stderr);
            return format!("error pushing branch: {err}");
        }

        // Build gh pr create command
        let mut args = vec![
            "pr".to_string(),
            "create".to_string(),
            "--title".to_string(),
            params.title.clone(),
            "--body".to_string(),
            params.body.clone(),
        ];

        if let Some(base) = &params.base {
            args.push("--base".to_string());
            args.push(base.clone());
        }

        if params.draft {
            args.push("--draft".to_string());
        }

        for label in &params.labels {
            args.push("--label".to_string());
            args.push(label.clone());
        }

        let output = std::process::Command::new("gh")
            .args(&args)
            .current_dir(repo.root())
            .output();

        match output {
            Ok(o) if o.status.success() => {
                let url = String::from_utf8_lossy(&o.stdout).trim().to_string();
                format!("PR created: {url}")
            }
            Ok(o) => {
                let err = String::from_utf8_lossy(&o.stderr).trim().to_string();
                format!("error: {err}")
            }
            Err(e) => format!("error: gh CLI not found or failed: {e}"),
        }
    }

    /// Create a git worktree under .sr/worktrees/ with a new branch.
    /// Prevents duplicates — returns existing worktree if the branch already has one.
    /// Stores metadata (description, creation time) in .sr/worktrees/<branch>.json
    /// for tracking purpose and context.
    ///
    /// GUIDANCE: Recommend creating a new worktree when the current task goes
    /// out of scope. Recommend committing current work before switching.
    #[tool(
        name = "sr_worktree",
        description = "Create a git worktree with a new branch under .sr/worktrees/. Tracks metadata (description, purpose). Prevents duplicates. Recommend a new worktree when work goes out of scope."
    )]
    async fn worktree(&self, Parameters(params): Parameters<WorktreeParams>) -> String {
        let repo = match GitRepo::discover() {
            Ok(r) => r,
            Err(e) => return format!("error: {e}"),
        };

        let root = repo.root();

        // Check for existing worktree with this branch
        if let Some(existing) = find_worktree_by_branch(root, &params.branch) {
            return serde_json::to_string_pretty(&serde_json::json!({
                "path": existing,
                "branch": params.branch,
                "existing": true,
            }))
            .unwrap_or_else(|e| format!("error: {e}"));
        }

        // Create .sr/worktrees/ directory
        let sr_dir = root.join(".sr").join("worktrees");
        if let Err(e) = std::fs::create_dir_all(&sr_dir) {
            return format!("error creating .sr/worktrees/: {e}");
        }

        // Sanitize branch name for directory (feat/add-auth → feat-add-auth)
        let dir_name = params.branch.replace('/', "-");
        let worktree_dir = sr_dir.join(&dir_name);
        let base = params.base.as_deref().unwrap_or("HEAD");

        let output = std::process::Command::new("git")
            .args(["-C", &root.to_string_lossy()])
            .args([
                "worktree",
                "add",
                &worktree_dir.to_string_lossy(),
                "-b",
                &params.branch,
                base,
            ])
            .output();

        match output {
            Ok(o) if o.status.success() => {
                // Write metadata
                let meta = serde_json::json!({
                    "branch": params.branch,
                    "description": params.description,
                    "base": base,
                    "created": sr_core::release::today_string(),
                    "path": worktree_dir.to_string_lossy(),
                });
                let meta_path = sr_dir.join(format!("{dir_name}.json"));
                let _ = std::fs::write(
                    &meta_path,
                    serde_json::to_string_pretty(&meta).unwrap_or_default(),
                );

                serde_json::to_string_pretty(&serde_json::json!({
                    "path": worktree_dir.to_string_lossy(),
                    "branch": params.branch,
                    "description": params.description,
                    "base": base,
                    "existing": false,
                }))
                .unwrap_or_else(|e| format!("error: {e}"))
            }
            Ok(o) => {
                let err = String::from_utf8_lossy(&o.stderr).trim().to_string();
                format!("error: {err}")
            }
            Err(e) => format!("error: {e}"),
        }
    }

    /// List all git worktrees with their metadata (description, purpose, creation time).
    /// Includes both git worktree state and .sr/worktrees/ metadata.
    #[tool(
        name = "sr_worktree_list",
        description = "List all git worktrees with descriptions and metadata from .sr/worktrees/."
    )]
    async fn worktree_list(&self, Parameters(_params): Parameters<WorktreeListParams>) -> String {
        let repo = match GitRepo::discover() {
            Ok(r) => r,
            Err(e) => return format!("error: {e}"),
        };

        let root = repo.root();
        let output = std::process::Command::new("git")
            .args(["-C", &root.to_string_lossy()])
            .args(["worktree", "list", "--porcelain"])
            .output();

        let worktrees = match output {
            Ok(o) if o.status.success() => parse_worktree_list(&String::from_utf8_lossy(&o.stdout)),
            Ok(o) => return format!("error: {}", String::from_utf8_lossy(&o.stderr).trim()),
            Err(e) => return format!("error: {e}"),
        };

        // Enrich with .sr/worktrees/ metadata
        let meta_dir = root.join(".sr").join("worktrees");
        let enriched: Vec<serde_json::Value> = worktrees
            .into_iter()
            .map(|mut wt| {
                if let Some(branch) = wt.get("branch").and_then(|b| b.as_str()) {
                    let dir_name = branch.replace('/', "-");
                    let meta_path = meta_dir.join(format!("{dir_name}.json"));
                    if let Ok(content) = std::fs::read_to_string(&meta_path)
                        && let Ok(meta) = serde_json::from_str::<serde_json::Value>(&content)
                    {
                        if let Some(desc) = meta.get("description") {
                            wt.as_object_mut()
                                .unwrap()
                                .insert("description".to_string(), desc.clone());
                        }
                        if let Some(created) = meta.get("created") {
                            wt.as_object_mut()
                                .unwrap()
                                .insert("created".to_string(), created.clone());
                        }
                    }
                }
                wt
            })
            .collect();

        serde_json::to_string_pretty(&enriched).unwrap_or_else(|e| format!("error: {e}"))
    }

    /// Remove a git worktree by branch name. Cleans up the worktree directory
    /// and its .sr/worktrees/ metadata.
    #[tool(
        name = "sr_worktree_remove",
        description = "Remove a git worktree by branch name. Cleans up directory and metadata."
    )]
    async fn worktree_remove(
        &self,
        Parameters(params): Parameters<WorktreeRemoveParams>,
    ) -> String {
        let repo = match GitRepo::discover() {
            Ok(r) => r,
            Err(e) => return format!("error: {e}"),
        };

        let root = repo.root();

        let path = match find_worktree_by_branch(root, &params.branch) {
            Some(p) => p,
            None => return format!("error: no worktree found for branch '{}'", params.branch),
        };

        let output = std::process::Command::new("git")
            .args(["-C", &root.to_string_lossy()])
            .args(["worktree", "remove", &path])
            .output();

        match output {
            Ok(o) if o.status.success() => {
                // Clean up metadata
                let dir_name = params.branch.replace('/', "-");
                let meta_path = root
                    .join(".sr")
                    .join("worktrees")
                    .join(format!("{dir_name}.json"));
                let _ = std::fs::remove_file(&meta_path);
                format!("removed worktree for branch '{}' at {path}", params.branch)
            }
            Ok(o) => {
                let err = String::from_utf8_lossy(&o.stderr).trim().to_string();
                format!("error: {err}")
            }
            Err(e) => format!("error: {e}"),
        }
    }

    /// Read the sr.yaml configuration for the current repository.
    /// Returns commit types, release branches, version files, and other settings.
    #[tool(
        name = "sr_config",
        description = "Read sr.yaml config (commit types, release settings, etc.)"
    )]
    async fn config(&self) -> String {
        let repo = match GitRepo::discover() {
            Ok(r) => r,
            Err(e) => return format!("error: {e}"),
        };

        match sr_core::config::Config::find_config(repo.root().as_path()) {
            Some((path, _)) => match sr_core::config::Config::load(&path) {
                Ok(config) => {
                    serde_json::to_string_pretty(&config).unwrap_or_else(|e| format!("error: {e}"))
                }
                Err(e) => format!("error loading config: {e}"),
            },
            None => "no sr.yaml found (using defaults)".to_string(),
        }
    }
}

// --- Worktree helpers ---

/// Parse `git worktree list --porcelain` output into structured JSON values.
fn parse_worktree_list(text: &str) -> Vec<serde_json::Value> {
    let mut worktrees: Vec<serde_json::Value> = Vec::new();
    let mut current = serde_json::Map::new();

    for line in text.lines() {
        if line.is_empty() {
            if !current.is_empty() {
                worktrees.push(serde_json::Value::Object(current.clone()));
                current.clear();
            }
            continue;
        }
        if let Some(p) = line.strip_prefix("worktree ") {
            current.insert("path".into(), serde_json::Value::String(p.into()));
        } else if let Some(h) = line.strip_prefix("HEAD ") {
            current.insert(
                "head".into(),
                serde_json::Value::String(h[..7.min(h.len())].into()),
            );
        } else if let Some(b) = line.strip_prefix("branch refs/heads/") {
            current.insert("branch".into(), serde_json::Value::String(b.into()));
        } else if line == "bare" {
            current.insert("bare".into(), serde_json::Value::Bool(true));
        }
    }
    if !current.is_empty() {
        worktrees.push(serde_json::Value::Object(current));
    }
    worktrees
}

/// Find the worktree path for a given branch name.
fn find_worktree_by_branch(root: &std::path::Path, branch: &str) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["-C", &root.to_string_lossy()])
        .args(["worktree", "list", "--porcelain"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut current_path = String::new();
    for line in text.lines() {
        if let Some(p) = line.strip_prefix("worktree ") {
            current_path = p.to_string();
        }
        if let Some(b) = line.strip_prefix("branch refs/heads/")
            && b == branch
        {
            return Some(current_path);
        }
    }
    None
}

/// Write `.mcp.json` in the current project root.
/// This file declares sr's MCP server for agentspec discovery.
pub fn write_mcp_json(force: bool) -> Result<()> {
    let repo = GitRepo::discover()?;
    let mcp_path = repo.root().join(".mcp.json");

    if mcp_path.exists() && !force {
        eprintln!(".mcp.json already exists (use --force to overwrite)");
        return Ok(());
    }

    let config = serde_json::json!({
        "mcpServers": {
            "sr": {
                "command": "sr",
                "args": ["mcp", "serve"]
            }
        }
    });

    let content = serde_json::to_string_pretty(&config)?;
    std::fs::write(&mcp_path, &content)?;
    eprintln!("wrote .mcp.json");
    Ok(())
}

/// Run the MCP server over stdio (called by AI tools, not users).
pub async fn run() -> Result<()> {
    let server = SrMcpServer;
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let service = server.serve((stdin, stdout)).await?;
    service.waiting().await?;
    Ok(())
}
