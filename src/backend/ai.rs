/// AI module: AI provider integration, config, helpers.
pub mod provider {
    // Re-export provider functions
    // Note: keep small facade to avoid exposing internals directly
    pub use crate::backend::ai_provider::*;
}

pub mod ai {
    // AI provider config, API key logic, doc comments
    // ...
}
