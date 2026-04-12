use anyhow::{Result, bail};
use sr_core::git::GitRepo;
use std::io::{self, Write};

#[derive(Debug, clap::Args)]
pub struct CommitArgs {
    /// Commit message
    #[arg(short, long)]
    pub message: Option<String>,

    /// Only commit staged changes (don't auto-stage)
    #[arg(short, long)]
    pub staged_only: bool,

    /// Display what would be committed without executing
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Skip confirmation prompts
    #[arg(short, long)]
    pub yes: bool,
}

pub async fn run(args: &CommitArgs) -> Result<()> {
    let repo = GitRepo::discover()?;

    let message = match &args.message {
        Some(m) => m.clone(),
        None => bail!("commit message required (-m). For AI-generated messages, use sr via MCP"),
    };

    if !args.staged_only {
        // Show what will be staged and confirm
        let status = repo.status_porcelain()?;
        if status.trim().is_empty() {
            bail!("no changes to commit");
        }

        if !args.yes {
            eprintln!("the following changes will be staged (git add -A):\n");
            for line in status.lines() {
                eprintln!("  {line}");
            }
            eprintln!();
            eprint!("stage all and commit? [y/N] ");
            io::stderr().flush()?;

            let mut answer = String::new();
            io::stdin().read_line(&mut answer)?;
            if !answer.trim().eq_ignore_ascii_case("y") {
                bail!("cancelled — use -s to commit only staged changes, or -y to skip prompts");
            }
        }

        let s = std::process::Command::new("git")
            .args(["-C", &repo.root().to_string_lossy()])
            .args(["add", "-A"])
            .status()?;
        if !s.success() {
            bail!("failed to stage files");
        }
    }

    if !repo.has_staged_changes()? {
        bail!("no staged changes to commit");
    }

    if args.dry_run {
        eprintln!("would commit: {message}");
        let stat = repo.diff_cached_stat()?;
        eprintln!("{stat}");
        return Ok(());
    }

    repo.commit(&message)?;
    let sha = repo.head_short()?;
    println!("{sha}  {message}");

    Ok(())
}
