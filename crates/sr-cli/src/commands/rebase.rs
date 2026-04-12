use anyhow::{Result, bail};
use sr_core::git::GitRepo;
use std::io::{self, Write};

#[derive(Debug, clap::Args)]
pub struct RebaseArgs {
    /// Number of recent commits to rebase (default: since last tag)
    #[arg(long)]
    pub last: Option<usize>,

    /// Skip confirmation prompt
    #[arg(short, long)]
    pub yes: bool,
}

pub async fn run(args: &RebaseArgs) -> Result<()> {
    let repo = GitRepo::discover()?;

    let count = match args.last {
        Some(n) => n,
        None => {
            let c = repo.commits_since_last_tag()?;
            if c == 0 {
                bail!("no commits found to rebase");
            }
            c
        }
    };

    if !args.yes {
        let log = repo.recent_commits(count)?;
        eprintln!("interactive rebase will rewrite {count} commit(s):\n");
        for line in log.lines() {
            eprintln!("  {line}");
        }
        eprintln!();
        eprint!("this rewrites history. continue? [y/N] ");
        io::stderr().flush()?;

        let mut answer = String::new();
        io::stdin().read_line(&mut answer)?;
        if !answer.trim().eq_ignore_ascii_case("y") {
            bail!("cancelled");
        }
    }

    let status = std::process::Command::new("git")
        .args(["-C", &repo.root().to_string_lossy()])
        .args(["rebase", "-i", &format!("HEAD~{count}")])
        .status()?;

    if !status.success() {
        bail!("rebase failed");
    }

    Ok(())
}
