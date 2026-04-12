use anyhow::Result;
use sr_core::git::GitRepo;

#[derive(Debug, clap::Args)]
pub struct PrArgs {
    /// PR title (auto-generated from branch name if omitted)
    #[arg(short, long)]
    pub title: Option<String>,

    /// PR body (auto-generated from commit log if omitted)
    #[arg(short, long)]
    pub body: Option<String>,

    /// Create as draft PR
    #[arg(short, long)]
    pub draft: bool,
}

pub async fn run(args: &PrArgs) -> Result<()> {
    let repo = GitRepo::discover()?;

    let config = sr_core::config::Config::find_config(repo.root().as_path())
        .map(|(path, _)| sr_core::config::Config::load(&path))
        .transpose()?
        .unwrap_or_default();
    let base = config
        .release
        .branches
        .first()
        .cloned()
        .unwrap_or_else(|| "main".to_string());

    let branch = repo.current_branch()?;

    let title = args
        .title
        .clone()
        .unwrap_or_else(|| branch.replace(['/', '-', '_'], " "));

    let body = match &args.body {
        Some(b) => b.clone(),
        None => {
            let log = repo.log_range(&format!("{base}..HEAD"), None)?;
            format!("## Commits\n\n{log}")
        }
    };

    let mut cmd = std::process::Command::new("gh");
    cmd.args(["pr", "create", "--title", &title, "--body", &body, "--base", &base]);

    if args.draft {
        cmd.arg("--draft");
    }

    cmd.current_dir(repo.root()).status()?;

    Ok(())
}
