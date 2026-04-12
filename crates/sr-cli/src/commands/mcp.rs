use anyhow::Result;
use rmcp::{ServiceExt, tool, tool_router};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::schemars::{self, JsonSchema};
use serde::Deserialize;
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
    /// Extended description with motivation and context. Optional.
    pub body: Option<String>,
    /// Footer lines (e.g. "BREAKING CHANGE: ...", "Closes #123"). Optional.
    pub footer: Option<String>,
    /// Specific files to include in this commit. Empty = all staged files.
    #[serde(default)]
    pub files: Vec<String>,
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
    #[tool(name = "sr_status", description = "Get repository status with file fingerprints. Call this first to see what changed.")]
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

    /// Get the diff for changed files. Use `staged: true` for only staged changes.
    /// Specify `files` to get diff for specific files only (reduces context size).
    #[tool(name = "sr_diff", description = "Get git diff output. Specify files to limit scope and save tokens.")]
    async fn diff(&self, Parameters(params): Parameters<DiffParams>) -> String {
        let repo = match GitRepo::discover() {
            Ok(r) => r,
            Err(e) => return format!("error: {e}"),
        };

        if params.files.is_empty() {
            // Full diff
            let diff = if params.staged {
                repo.diff_cached()
            } else {
                repo.diff_head()
            };
            match diff {
                Ok(d) if d.trim().is_empty() => "No changes.".to_string(),
                Ok(d) => d,
                Err(e) => format!("error: {e}"),
            }
        } else {
            // Per-file diffs (token-efficient)
            let mut result = String::new();
            for file in &params.files {
                let args = if params.staged {
                    vec!["diff", "--cached", "--", file]
                } else {
                    vec!["diff", "HEAD", "--", file]
                };
                let output = std::process::Command::new("git")
                    .args(["-C", &repo.root().to_string_lossy()])
                    .args(&args)
                    .output();
                match output {
                    Ok(o) if o.status.success() => {
                        let diff = String::from_utf8_lossy(&o.stdout);
                        if !diff.trim().is_empty() {
                            result.push_str(&diff);
                            result.push('\n');
                        }
                    }
                    Ok(_) => result.push_str(&format!("# {file}: no diff\n")),
                    Err(e) => result.push_str(&format!("# {file}: error: {e}\n")),
                }
            }
            if result.is_empty() {
                "No changes for specified files.".to_string()
            } else {
                result
            }
        }
    }

    /// Get the commit log. Use `range` for specific ranges (e.g. "main..HEAD" for PR commits)
    /// or `count` for recent N commits.
    #[tool(name = "sr_log", description = "Get commit log. Use range for PR commits or count for recent history.")]
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
    #[tool(name = "sr_stage", description = "Stage files for commit. Use [\".\"] for all changes. Modifies the index.")]
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
    #[tool(name = "sr_commit", description = "Create a conventional commit. Stage files first with sr_stage.")]
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
        let header = match &params.scope {
            Some(scope) => format!("{}({}): {}", params.r#type, scope, params.description),
            None => format!("{}: {}", params.r#type, params.description),
        };

        let mut message = header.clone();
        if let Some(body) = &params.body {
            message.push_str("\n\n");
            message.push_str(body);
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
    #[tool(name = "sr_branch", description = "Get current branch or create a new one.")]
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

    /// Read the sr.yaml configuration for the current repository.
    /// Returns commit types, release branches, version files, and other settings.
    #[tool(name = "sr_config", description = "Read sr.yaml config (commit types, release settings, etc.)")]
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

/// Run the MCP server over stdio.
pub async fn run() -> Result<()> {
    eprintln!("sr mcp server running on stdio");
    eprintln!("tools: sr_status, sr_diff, sr_log, sr_stage, sr_commit, sr_branch, sr_config");
    eprintln!();
    eprintln!("register: agentspec mcp add sr --command sr --args \"mcp serve\"");
    eprintln!("press ctrl+c to stop");

    let server = SrMcpServer;
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let service = server.serve((stdin, stdout)).await?;
    service.waiting().await?;
    Ok(())
}
