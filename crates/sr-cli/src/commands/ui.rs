use crossterm::style::Stylize;
use sr_core::ai::{AiEvent, AiUsage};
use sr_core::ai::services::commit::CommitPlan;
use std::collections::HashMap;

pub use agentspec_ui::{
    confirm, format_tokens, header, info, phase_ok, spinner, spinner_done, tool_call,
};

pub fn display_plan(
    plan: &CommitPlan,
    statuses: &HashMap<String, char>,
    cache_label: Option<&str>,
) {
    let count = plan.commits.len();
    let count_str = format!("{count} commit{}", if count == 1 { "" } else { "s" });
    let label = match cache_label {
        Some(l) => format!("{count_str} · {l}"),
        None => count_str,
    };

    println!();
    println!("  {} {}", "COMMIT PLAN".bold(), format!("· {label}").dim());
    let rule = "─".repeat(50);
    println!("  {}", rule.as_str().dim());

    for (i, commit) in plan.commits.iter().enumerate() {
        let order = commit.order.unwrap_or(i as u32 + 1);
        let idx = format!("[{order}]");

        println!();
        println!(
            "  {} {}",
            idx.as_str().cyan().bold(),
            commit.message.as_str().bold()
        );

        if let Some(body) = &commit.body {
            if !body.is_empty() {
                for line in body.lines() {
                    println!("   {}  {}", "│".dim(), line.dim());
                }
            }
        }

        if let Some(footer) = &commit.footer {
            if !footer.is_empty() {
                println!("   {}", "│".dim());
                for line in footer.lines() {
                    println!("   {}  {}", "│".dim(), line.yellow());
                }
            }
        }

        println!("   {}", "│".dim());

        let fc = commit.files.len();
        if fc == 0 {
            println!("   {} {}", "└─".dim(), "(no files)".dim());
        } else {
            for (j, file) in commit.files.iter().enumerate() {
                let is_last = j == fc - 1;
                let connector = if is_last { "└─" } else { "├─" };
                let status_char = statuses.get(file).copied().unwrap_or('~');
                let status_styled = match status_char {
                    'A' => format!("{}", "A".green()),
                    'D' => format!("{}", "D".red()),
                    'M' => format!("{}", "M".yellow()),
                    'R' => format!("{}", "R".blue()),
                    _ => format!("{}", "·".dim()),
                };
                println!("   {} {} {}", connector.dim(), status_styled, file);
            }
        }
    }

    println!();
    println!("  {}", rule.as_str().dim());
}

pub fn commit_start(index: usize, total: usize, message: &str) {
    println!();
    println!(
        "  {} {}",
        format!("[{index}/{total}]").as_str().cyan().bold(),
        message.bold()
    );
}

#[allow(dead_code)]
pub fn file_staged(file: &str, success: bool) {
    if success {
        println!("    {} {}", "✓".green(), file.dim());
    } else {
        println!("    {} {} {}", "⚠".yellow(), file, "(not found)".dim());
    }
}

pub fn commit_created(sha: &str) {
    println!("    {} {}", "→".green().bold(), sha.green());
}

pub fn commit_skipped() {
    println!("    {} {}", "−".yellow(), "skipped (no staged files)".dim());
}

pub fn commit_failed(reason: &str) {
    println!(
        "    {} {} {}",
        "✗".red().bold(),
        "failed:".red(),
        reason.dim()
    );
}

pub fn summary(commits: &[(String, String)]) {
    let count = commits.len();
    println!();
    println!(
        "  {} {} commit{} created",
        "✓".green().bold(),
        count.to_string().as_str().bold(),
        if count == 1 { "" } else { "s" }
    );
    println!();
    for (sha, msg) in commits {
        println!("    {}  {}", sha.as_str().dim(), msg);
    }
    println!();
}

pub fn invalid_messages(invalid: &[(usize, String, String)]) {
    println!();
    println!(
        "  {} {}",
        "⚠".yellow().bold(),
        format!(
            "{} commit message{} failed validation:",
            invalid.len(),
            if invalid.len() == 1 { "" } else { "s" }
        )
        .yellow()
    );
    for (idx, msg, reason) in invalid {
        println!(
            "    {} {} — {}",
            format!("[{idx}]").cyan(),
            msg,
            reason.as_str().dim()
        );
    }
    println!();
}

pub fn failed_commits(failed: &[(usize, String, String)]) {
    println!(
        "  {} {}",
        "⚠".yellow().bold(),
        format!(
            "{} commit{} failed:",
            failed.len(),
            if failed.len() == 1 { "" } else { "s" }
        )
        .yellow()
    );
    for (idx, msg, reason) in failed {
        println!(
            "    {} {} — {}",
            format!("[{idx}]").cyan(),
            msg,
            reason.as_str().dim()
        );
    }
    println!();
}

pub fn display_rebase_plan(plan: &sr_core::ai::services::rebase::ReorganizePlan) {
    println!();
    println!(
        "  {} {}",
        "REBASE PLAN".bold(),
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

        if let Some(body) = &commit.body {
            if !body.is_empty() {
                for line in body.lines() {
                    println!("   {}  {}", "│".dim(), line.dim());
                }
            }
        }
    }

    println!();
    println!("  {}", rule.as_str().dim());
    println!();
}

/// Format detail string for spinner_done, including usage if available.
pub fn format_done_detail(
    commit_count: usize,
    extra: &str,
    usage: &Option<AiUsage>,
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
                format_tokens(u.input_tokens),
                format_tokens(u.output_tokens),
                cost
            )
        }
        None => String::new(),
    };
    format!("{commits}{extra_part}{usage_part}")
}

/// Spawn a background task that renders AI events above a spinner.
pub fn spawn_event_handler(
    spinner: &indicatif::ProgressBar,
) -> (
    tokio::sync::mpsc::UnboundedSender<AiEvent>,
    tokio::task::JoinHandle<()>,
) {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let pb = spinner.clone();
    let handle = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                AiEvent::ToolCall { input, .. } => tool_call(&pb, &input),
            }
        }
    });
    (tx, handle)
}
