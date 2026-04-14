mod commands;

use std::path::Path;
use std::process::ExitCode;

use clap::{CommandFactory, Parser, Subcommand};
use sr_core::changelog::DefaultChangelogFormatter;
use sr_core::commit::ConfiguredCommitParser;
use sr_core::config::{Config, DEFAULT_CONFIG_FILE, LEGACY_CONFIG_FILE, VersioningMode};
use sr_core::error::ReleaseError;
use sr_core::github::GitHubProvider;
use sr_core::native_git::NativeGitRepository;
use sr_core::release::{ReleaseStrategy, TrunkReleaseStrategy};

#[derive(Parser)]
#[command(name = "sr", about = "Release engineering CLI", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute a release (trunk flow: tag + GitHub release)
    Release {
        /// Target a specific package in a monorepo
        #[arg(long, short)]
        package: Option<String>,

        /// Release channel (e.g. canary, rc, stable) — overrides config fields
        #[arg(long, short)]
        channel: Option<String>,

        /// Preview what would happen without making changes
        #[arg(long)]
        dry_run: bool,

        /// Glob patterns for artifact files to upload to the release (repeatable)
        #[arg(long = "artifacts")]
        artifacts: Vec<String>,

        /// Re-release the current tag (use when a previous release partially failed)
        #[arg(long)]
        force: bool,

        /// Additional file globs to stage in the release commit (repeatable, e.g. Cargo.lock)
        #[arg(long = "stage-files")]
        stage_files: Vec<String>,

        /// Pre-release identifier (e.g. alpha, beta, rc). Produces versions like 1.2.0-alpha.1
        #[arg(long)]
        prerelease: Option<String>,

        /// Sign tags with GPG/SSH (git tag -s)
        #[arg(long)]
        sign_tags: bool,

        /// Create GitHub release as a draft (requires manual publishing)
        #[arg(long)]
        draft: bool,
    },

    /// Show repo status: unreleased commits, next version, changelog preview, open PRs
    Status {
        /// Target a specific package in a monorepo
        #[arg(long, short)]
        package: Option<String>,

        /// Output format
        #[arg(long, default_value = "human")]
        format: PlanFormat,
    },

    /// Validate and display resolved configuration
    Config {
        /// Show the fully resolved config with defaults applied
        #[arg(long)]
        resolved: bool,
    },

    /// Create default configuration files (sr.yaml + .mcp.json)
    Init {
        /// Overwrite config files if they already exist
        #[arg(long)]
        force: bool,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: clap_complete::Shell,
    },

    /// MCP server
    Mcp {
        #[command(subcommand)]
        command: McpCommands,
    },

    /// Update sr to the latest version
    Update,

    /// Show migration guide
    Migrate,
}

#[derive(Subcommand)]
enum McpCommands {
    /// Start MCP server over stdio
    Serve,
}

#[derive(Clone, clap::ValueEnum)]
enum PlanFormat {
    Human,
    Json,
}

use sr_core::release::NoopVcsProvider;

fn build_local_strategy(
    config: Config,
    force: bool,
) -> anyhow::Result<
    TrunkReleaseStrategy<
        NativeGitRepository,
        NoopVcsProvider,
        ConfiguredCommitParser,
        DefaultChangelogFormatter,
    >,
> {
    let git = NativeGitRepository::open(Path::new("."))?;
    let parser =
        ConfiguredCommitParser::new(config.commit.types.clone(), config.commit.pattern.clone());
    let types = config.commit.types.clone();
    let breaking_section = config.commit.breaking_section.clone();
    let misc_section = config.commit.misc_section.clone();
    let formatter = DefaultChangelogFormatter::new(
        config.release.changelog.template.clone(),
        types,
        breaking_section,
        misc_section,
    );
    Ok(TrunkReleaseStrategy {
        git,
        vcs: NoopVcsProvider,
        parser,
        formatter,
        config,
        force,
    })
}

fn build_full_strategy(
    config: Config,
    force: bool,
) -> anyhow::Result<
    TrunkReleaseStrategy<
        NativeGitRepository,
        GitHubProvider,
        ConfiguredCommitParser,
        DefaultChangelogFormatter,
    >,
> {
    let git = NativeGitRepository::open(Path::new("."))?;
    let (hostname, owner, repo) = git.parse_remote_full()?;

    let token = std::env::var("GH_TOKEN")
        .or_else(|_| std::env::var("GITHUB_TOKEN"))
        .map_err(|_| anyhow::anyhow!("neither GH_TOKEN nor GITHUB_TOKEN is set"))?;

    let git = git.with_http_auth(hostname.clone(), token.clone());
    let vcs = GitHubProvider::new(owner, repo, hostname, token);
    let parser =
        ConfiguredCommitParser::new(config.commit.types.clone(), config.commit.pattern.clone());
    let types = config.commit.types.clone();
    let breaking_section = config.commit.breaking_section.clone();
    let misc_section = config.commit.misc_section.clone();
    let formatter = DefaultChangelogFormatter::new(
        config.release.changelog.template.clone(),
        types,
        breaking_section,
        misc_section,
    );

    Ok(TrunkReleaseStrategy {
        git,
        vcs,
        parser,
        formatter,
        config,
        force,
    })
}

fn is_no_release_error(err: &anyhow::Error) -> bool {
    if let Some(re) = err.downcast_ref::<ReleaseError>() {
        matches!(
            re,
            ReleaseError::NoCommits { .. } | ReleaseError::NoBump { .. }
        )
    } else {
        false
    }
}

fn load_config_for_package(package: Option<&str>) -> anyhow::Result<Config> {
    let config_path = resolve_config_path();
    let mut config = Config::load(&config_path)?;

    if config.release.versioning == VersioningMode::Fixed && !config.packages.is_empty() {
        if let Some(name) = package {
            anyhow::bail!(
                "--package '{name}' is not supported with `versioning: fixed` — \
                 all packages are released together"
            );
        }
        return Ok(config.resolve_fixed());
    }

    match package {
        Some(name) => {
            let pkg = config.find_package(name)?;
            Ok(config.resolve_package(pkg))
        }
        None => {
            if config.release.version_files.is_empty() {
                config.release.version_files =
                    sr_core::version_files::detect_version_files(Path::new("."));
            }
            Ok(config)
        }
    }
}

fn resolve_config_path() -> std::path::PathBuf {
    match Config::find_config(Path::new(".")) {
        Some((path, is_legacy)) => {
            if is_legacy {
                eprintln!(
                    "warning: {} is deprecated, rename to {} (legacy support will be removed in a future release)",
                    LEGACY_CONFIG_FILE, DEFAULT_CONFIG_FILE,
                );
            }
            path
        }
        None => std::path::PathBuf::from(DEFAULT_CONFIG_FILE),
    }
}

fn self_update() -> anyhow::Result<()> {
    eprintln!("current version: {}", env!("CARGO_PKG_VERSION"));

    match agentspec_update::self_update("urmzd/sr", env!("CARGO_PKG_VERSION"), "sr")? {
        agentspec_update::UpdateResult::AlreadyUpToDate => {
            eprintln!("already up to date");
        }
        agentspec_update::UpdateResult::Updated { from, to } => {
            eprintln!("updated: {from} → {to}");
        }
    }

    Ok(())
}

fn print_migration_guide() {
    print!("{}", include_str!("../docs/migration.md"));
}

async fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { force } => {
            // sr.yaml
            let path = Path::new(DEFAULT_CONFIG_FILE);
            if !path.exists() || force {
                let detected = sr_core::version_files::detect_version_files(Path::new("."));
                if !detected.is_empty() {
                    for f in &detected {
                        eprintln!("detected version file: {f}");
                    }
                }
                let template = sr_core::config::default_config_template(&detected);
                std::fs::write(path, template)?;
                eprintln!("wrote {DEFAULT_CONFIG_FILE}");
            } else {
                eprintln!(
                    "{DEFAULT_CONFIG_FILE} already exists (skipping, use --force to overwrite)"
                );
            }

            // .mcp.json
            commands::mcp::write_mcp_json(force)?;

            // .gitignore — ensure .sr/ is listed
            let gitignore = Path::new(".gitignore");
            let needs_entry = if gitignore.exists() {
                let content = std::fs::read_to_string(gitignore)?;
                !content
                    .lines()
                    .any(|l| l.trim() == ".sr" || l.trim() == ".sr/")
            } else {
                true
            };
            if needs_entry {
                use std::io::Write;
                let mut f = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(gitignore)?;
                writeln!(f, "\n# sr cache and worktrees\n.sr/")?;
                eprintln!("added .sr/ to .gitignore");
            }

            Ok(())
        }

        Commands::Config { resolved } => {
            let config_path = resolve_config_path();
            let config = Config::load(&config_path)?;
            if resolved {
                let yaml = serde_yaml_ng::to_string(&config)?;
                print!("{yaml}");
            } else if config_path.exists() {
                let raw = std::fs::read_to_string(&config_path)?;
                print!("{raw}");
            } else {
                eprintln!("no config file found; showing defaults");
                let yaml = serde_yaml_ng::to_string(&config)?;
                print!("{yaml}");
            }
            Ok(())
        }

        Commands::Status { package, format } => {
            let config = load_config_for_package(package.as_deref())?;
            let tag_prefix = config.release.tag_prefix.clone();

            let git = NativeGitRepository::open(Path::new("."))?;
            let branch_output = std::process::Command::new("git")
                .args(["branch", "--show-current"])
                .output()?;
            let branch = String::from_utf8_lossy(&branch_output.stdout)
                .trim()
                .to_string();

            let formatter = DefaultChangelogFormatter::new(
                config.release.changelog.template.clone(),
                config.commit.types.clone(),
                config.commit.breaking_section.clone(),
                config.commit.misc_section.clone(),
            );
            let strategy = build_local_strategy(config, false)?;
            let plan_result = strategy.plan();

            match format {
                PlanFormat::Json => match plan_result {
                    Ok(plan) => {
                        let repo_url = git
                            .parse_remote_full()
                            .ok()
                            .map(|(h, o, r)| format!("https://{h}/{o}/{r}"));
                        let today = sr_core::release::today_string();
                        let entry = sr_core::changelog::ChangelogEntry {
                            version: plan.next_version.to_string(),
                            date: today,
                            commits: plan.commits.clone(),
                            compare_url: None,
                            repo_url,
                        };
                        let changelog =
                            sr_core::changelog::ChangelogFormatter::format(&formatter, &[entry])?;
                        #[derive(serde::Serialize)]
                        struct StatusOutput<'a> {
                            branch: String,
                            #[serde(flatten)]
                            plan: &'a sr_core::release::ReleasePlan,
                            changelog: String,
                        }
                        let output = StatusOutput {
                            branch,
                            plan: &plan,
                            changelog,
                        };
                        println!("{}", serde_json::to_string_pretty(&output)?);
                    }
                    Err(e) => {
                        let msg = if matches!(
                            &e,
                            ReleaseError::NoCommits { .. } | ReleaseError::NoBump { .. }
                        ) {
                            "no unreleased changes"
                        } else {
                            "error"
                        };
                        println!("{{\"branch\":\"{branch}\",\"status\":\"{msg}\"}}");
                    }
                },
                PlanFormat::Human => {
                    println!("  Branch: {branch}");
                    match plan_result {
                        Ok(plan) => {
                            let current_tag = plan
                                .current_version
                                .as_ref()
                                .map(|v| format!("{tag_prefix}{v}"))
                                .unwrap_or_else(|| "(initial)".to_string());
                            println!("  Current: {current_tag}");
                            println!("  Next: {} ({})", plan.tag_name, plan.bump);
                            println!("  Commits: {}", plan.commits.len());
                            for commit in &plan.commits {
                                let scope = commit
                                    .scope
                                    .as_deref()
                                    .map(|s| format!("({s})"))
                                    .unwrap_or_default();
                                let breaking = if commit.breaking { " BREAKING" } else { "" };
                                println!(
                                    "    {}{scope}: {}{breaking}",
                                    commit.r#type, commit.description
                                );
                            }
                        }
                        Err(e) => match &e {
                            ReleaseError::NoCommits { .. } | ReleaseError::NoBump { .. } => {
                                println!("  No unreleased changes");
                            }
                            _ => println!("  Release: error — {e}"),
                        },
                    }
                    if let Ok((hostname, owner, repo_name)) = git.parse_remote_full()
                        && let Ok(token) =
                            std::env::var("GH_TOKEN").or_else(|_| std::env::var("GITHUB_TOKEN"))
                    {
                        let github = GitHubProvider::new(owner, repo_name, hostname, token);
                        if let Ok((ready, draft)) = github.count_open_prs() {
                            println!(
                                "  Open PRs: {} ({} ready, {} draft)",
                                ready + draft,
                                ready,
                                draft
                            );
                        }
                    }
                }
            }
            Ok(())
        }

        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            clap_complete::generate(shell, &mut cmd, "sr", &mut std::io::stdout());
            Ok(())
        }

        Commands::Release {
            package,
            channel,
            dry_run,
            artifacts,
            force,
            stage_files,
            prerelease,
            sign_tags,
            draft,
        } => {
            let mut config = load_config_for_package(package.as_deref())?;

            let channel_name = channel.or_else(|| config.release.default_channel.clone());
            if let Some(name) = &channel_name {
                config = config
                    .resolve_channel(name)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
            }
            config.release.artifacts.extend(artifacts);
            config.release.stage_files.extend(stage_files);
            if prerelease.is_some() {
                config.release.prerelease = prerelease;
            }
            if sign_tags {
                config.release.sign_tags = true;
            }
            if draft {
                config.release.draft = true;
            }

            let plan = match build_full_strategy(config.clone(), force) {
                Ok(strategy) => {
                    let plan = strategy.plan()?;
                    strategy.execute(&plan, dry_run)?;
                    plan
                }
                Err(e) => {
                    if dry_run {
                        eprintln!("warning: {e} (continuing dry-run without GitHub)");
                        let strategy = build_local_strategy(config, force)?;
                        let plan = strategy.plan()?;
                        strategy.execute(&plan, dry_run)?;
                        plan
                    } else {
                        return Err(e);
                    }
                }
            };
            #[derive(serde::Serialize)]
            struct ReleaseOutput {
                version: String,
                previous_version: String,
                tag: String,
                bump: String,
                floating_tag: String,
                commit_count: usize,
            }
            let output = ReleaseOutput {
                version: plan.next_version.to_string(),
                previous_version: plan
                    .current_version
                    .as_ref()
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                tag: plan.tag_name.clone(),
                bump: plan.bump.to_string(),
                floating_tag: plan.floating_tag_name.as_deref().unwrap_or("").to_string(),
                commit_count: plan.commits.len(),
            };
            println!("{}", serde_json::to_string(&output)?);
            Ok(())
        }

        Commands::Mcp { command } => match command {
            McpCommands::Serve => commands::mcp::run().await,
        },
        Commands::Update => self_update(),
        Commands::Migrate => {
            print_migration_guide();
            Ok(())
        }
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::from(0),
        Err(e) => {
            if is_no_release_error(&e) {
                eprintln!("{e:#}");
                ExitCode::from(2)
            } else {
                eprintln!("error: {e:#}");
                ExitCode::from(1)
            }
        }
    }
}
