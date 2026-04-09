use crate::ai::AiUsage;
use crate::commands::commit::CommitPlan;
use crossterm::style::Stylize;
use std::collections::HashMap;

// Re-export generic UI primitives from agentspec-ui.
pub use agentspec_ui::{
    confirm, format_tokens, header, info, is_tty, phase_ok, spinner, spinner_done, tool_call, warn,
};

/// Display token usage and cost.
pub fn usage(usage: &AiUsage) {
    agentspec_ui::usage(usage.input_tokens, usage.output_tokens, usage.cost_usd);
}

/// Display the commit plan with file statuses and optional cache label.
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

        if let Some(body) = &commit.body
            && !body.is_empty()
        {
            for line in body.lines() {
                println!("   {}  {}", "│".dim(), line.dim());
            }
        }

        if let Some(footer) = &commit.footer
            && !footer.is_empty()
        {
            println!("   {}", "│".dim());
            for line in footer.lines() {
                println!("   {}  {}", "│".dim(), line.yellow());
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

/// Print commit execution header.
pub fn commit_start(index: usize, total: usize, message: &str) {
    println!();
    println!(
        "  {} {}",
        format!("[{index}/{total}]").as_str().cyan().bold(),
        message.bold()
    );
}

/// Print a file staging result.
pub fn file_staged(file: &str, success: bool) {
    if success {
        println!("    {} {}", "✓".green(), file.dim());
    } else {
        println!("    {} {} {}", "⚠".yellow(), file, "(not found)".dim());
    }
}

/// Print commit created with short SHA.
pub fn commit_created(sha: &str) {
    println!("    {} {}", "→".green().bold(), sha.green());
}

/// Print commit skipped notice.
pub fn commit_skipped() {
    println!("    {} {}", "−".yellow(), "skipped (no staged files)".dim());
}

/// Print commit failed notice.
pub fn commit_failed(reason: &str) {
    println!(
        "    {} {} {}",
        "✗".red().bold(),
        "failed:".red(),
        reason.dim()
    );
}

/// Print final summary with commit list.
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

/// Display invalid commit messages found during pre-validation.
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

/// Display commits that failed during execution.
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
