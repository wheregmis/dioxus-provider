#![doc = include_str!("../README.md")]

// Core modules
pub mod cache;
pub mod errors;
pub mod global;
pub mod hooks;
pub mod injection;
mod log_utils;
pub mod mutation;
pub mod param_utils;
pub mod platform;
mod provider_state;
pub mod refresh;
pub mod types;

// Re-export commonly used items at crate root for convenience
pub use global::ProviderConfig;
pub use global::init;

pub mod prelude {
    //! The prelude exports all the most common types and functions for using dioxus-provider.

    // The main provider trait and the macro
    pub use crate::hooks::Provider;
    pub use dioxus_provider_macros::{mutation, provider};

    // The core hook for using providers
    pub use crate::hooks::use_provider;

    // Hooks for manual cache management
    pub use crate::hooks::use_clear_provider_cache;
    pub use crate::hooks::use_invalidate_provider;
    pub use crate::hooks::use_provider_cache;

    // The async state enum, needed for matching
    pub use crate::provider_state::{AsyncState, ProviderState};

    // Global initialization
    pub use crate::global::{ProviderConfig, init};

    // Dependency Injection
    pub use crate::injection::{
        clear_dependencies, has_dependency, inject, register_dependency,
    };

    // Mutation system - Manual Implementation Pattern
    pub use crate::mutation::{
        Mutation, MutationContext, MutationState, provider_cache_key, provider_cache_key_simple,
        use_mutation, use_optimistic_mutation,
    };

    // Error types
    pub use crate::errors::{
        ApiError, ApiResult, DatabaseError, DatabaseResult, ProviderError, ProviderResult,
        UserError, UserResult,
    };

    // Parameter utilities for custom types
    pub use crate::param_utils::IntoProviderParam;
}
