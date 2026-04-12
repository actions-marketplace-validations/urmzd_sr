use anyhow::Result;
use sr_core::ai::cache::{CacheManager, store};
use sr_core::ai::git::GitRepo;

#[derive(Debug, clap::Args)]
pub struct CacheArgs {
    #[command(subcommand)]
    pub command: CacheCommand,
}

#[derive(Debug, clap::Subcommand)]
pub enum CacheCommand {
    /// Show cached entries for this repo
    Status,
    /// Clear cached entries
    Clear {
        /// Clear cache for all repos, not just the current one
        #[arg(long)]
        all: bool,
    },
}

pub fn run(args: &CacheArgs) -> Result<()> {
    match &args.command {
        CacheCommand::Status => status(),
        CacheCommand::Clear { all } => clear(*all),
    }
}

fn status() -> Result<()> {
    let repo = GitRepo::discover()?;
    let repo_root = repo.root();

    let Some(dir) = store::cache_dir(repo_root) else {
        println!("Cache directory could not be resolved.");
        return Ok(());
    };

    let entries = store::list_entries(&dir)?;

    if entries.is_empty() {
        println!("No cached entries for this repository.");
        return Ok(());
    }

    let now = store::now_secs();

    println!("Cached commit plans ({} entries):", entries.len());
    println!();
    for entry in &entries {
        let age_secs = now.saturating_sub(entry.created_at);
        let age = format_age(age_secs);
        let file_count: usize = entry.plan.commits.iter().map(|c| c.files.len()).sum();
        let commit_count = entry.plan.commits.len();
        println!(
            "  {} — {} commit(s), {} file(s), backend={}, model={}, age={}",
            &entry.state_key[..12],
            commit_count,
            file_count,
            entry.backend,
            entry.model,
            age,
        );
    }

    println!();
    println!("Cache dir: {}", dir.display());

    Ok(())
}

fn clear(all: bool) -> Result<()> {
    if all {
        let count = store::clear_all()?;
        println!("Cleared {count} cached entries across all repositories.");
    } else {
        let repo = GitRepo::discover()?;
        let cm = CacheManager::new(repo.root(), false, None, "", "");
        match cm {
            Some(cm) => {
                let count = cm.clear()?;
                println!("Cleared {count} cached entries for this repository.");
            }
            None => {
                println!("Cache directory could not be resolved.");
            }
        }
    }
    Ok(())
}

fn format_age(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else {
        format!("{}h", secs / 3600)
    }
}
