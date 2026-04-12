use thiserror::Error;

#[derive(Debug, Error)]
pub enum SrAiError {
    #[error("not in a git repository")]
    NotAGitRepo,

    #[error("no changes to commit")]
    NoChanges,

    #[error("no commits in plan")]
    EmptyPlan,

    #[error("git command failed: {0}")]
    GitCommand(String),

    #[error("user cancelled")]
    Cancelled,
}
