//! Centralized prompt definitions for all AI-powered commands.
//!
//! Prompts and schemas are stored as template files under `prompts/` and
//! embedded at compile time via `include_str!`. Dynamic prompts use
//! Minijinja for rendering.
//!
//! Directory layout:
//! ```text
//! crates/sr-ai/src/prompts/
//! ├── commit/
//! │   ├── system.txt
//! │   ├── user.txt.j2
//! │   ├── patch.txt.j2
//! │   └── schema.json
//! ├── rebase/
//! │   ├── system.txt.j2
//! │   ├── user.txt.j2
//! │   └── schema.json
//! ├── review/
//! │   └── system.txt
//! ├── pr/
//! │   ├── system.txt
//! │   └── schema.json
//! ├── explain/
//! │   └── system.txt
//! ├── branch/
//! │   └── system.txt
//! └── ask/
//!     └── system.txt
//! ```

use super::services::commit::CommitPlan;

fn render(template: &str, ctx: &minijinja::Value) -> String {
    let env = minijinja::Environment::new();
    env.render_str(template, ctx).unwrap_or_else(|e| {
        eprintln!("warning: prompt template render failed: {e}");
        template.to_string()
    })
}

// ---------------------------------------------------------------------------
// Commit
// ---------------------------------------------------------------------------

pub mod commit {
    use super::*;

    pub const SCHEMA: &str = include_str!("commit/schema.json");

    const SYSTEM_TEMPLATE: &str = include_str!("commit/system.txt.j2");
    const USER_TEMPLATE: &str = include_str!("commit/user.txt.j2");
    const PATCH_TEMPLATE: &str = include_str!("commit/patch.txt.j2");

    pub fn system_prompt(commit_pattern: &str, type_names: &[&str]) -> String {
        let ctx = minijinja::context! {
            commit_pattern => commit_pattern,
            types_list => type_names.join(", "),
        };
        render(SYSTEM_TEMPLATE, &ctx)
    }

    pub fn user_prompt(staged_only: bool, git_root: &str, message: Option<&str>) -> String {
        let ctx = minijinja::context! {
            staged_only => staged_only,
            git_root => git_root,
            message => message,
        };
        render(USER_TEMPLATE, &ctx)
    }

    pub fn patch_prompt(
        staged_only: bool,
        git_root: &str,
        message: Option<&str>,
        existing_plan: &CommitPlan,
        unplaced_files: &[String],
        delta_summary: &str,
    ) -> String {
        let plan_json =
            serde_json::to_string_pretty(existing_plan).unwrap_or_else(|_| "{}".to_string());
        let ctx = minijinja::context! {
            staged_only => staged_only,
            git_root => git_root,
            message => message,
            plan_json => plan_json,
            unplaced_files => unplaced_files.join(", "),
            delta_summary => delta_summary,
        };
        render(PATCH_TEMPLATE, &ctx)
    }
}

// ---------------------------------------------------------------------------
// Rebase
// ---------------------------------------------------------------------------

pub mod rebase {
    use super::*;

    pub const SCHEMA: &str = include_str!("rebase/schema.json");

    const SYSTEM_TEMPLATE: &str = include_str!("rebase/system.txt.j2");
    const USER_TEMPLATE: &str = include_str!("rebase/user.txt.j2");

    pub fn system_prompt(commit_pattern: &str, type_names: &[&str]) -> String {
        let ctx = minijinja::context! {
            commit_pattern => commit_pattern,
            types_list => type_names.join(", "),
        };
        render(SYSTEM_TEMPLATE, &ctx)
    }

    pub fn user_prompt(log: &str, extra: Option<&str>) -> String {
        let ctx = minijinja::context! {
            log => log,
            extra => extra,
        };
        render(USER_TEMPLATE, &ctx)
    }
}

// ---------------------------------------------------------------------------
// Review
// ---------------------------------------------------------------------------

pub mod review {
    pub const SYSTEM_PROMPT: &str = include_str!("review/system.txt");
    pub const PR_SYSTEM_PROMPT: &str = include_str!("review/pr_system.txt");
}

// ---------------------------------------------------------------------------
// PR
// ---------------------------------------------------------------------------

pub mod pr {
    pub const SYSTEM_PROMPT: &str = include_str!("pr/system.txt");
    pub const SCHEMA: &str = include_str!("pr/schema.json");
}

// ---------------------------------------------------------------------------
// Branch
// ---------------------------------------------------------------------------

pub mod branch {
    pub const SYSTEM_PROMPT: &str = include_str!("branch/system.txt");
}
