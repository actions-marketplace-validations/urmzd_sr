pub mod backend;
pub mod cache;
pub mod error;
pub mod git;
pub mod prompts;
pub mod services;

// Re-export commonly used items from backend at this level
pub use backend::*;
