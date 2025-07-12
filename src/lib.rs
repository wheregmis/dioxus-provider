#![doc = include_str!("../README.md")]

// Core modules
pub mod cache;
pub mod errors;
pub mod global;
pub mod hooks;
pub mod injection;
pub mod mutation;
mod param_utils;
pub mod platform;
mod provider_state;
pub mod refresh;
pub mod types;

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
    pub use crate::cache::ProviderState;

    // Global initialization
    pub use crate::global::init_global_providers;

    // Dependency Injection
    pub use crate::injection::{
        clear_dependencies, has_dependency, init_dependency_injection, inject, register_dependency,
    };

    // Mutation system - Manual Implementation Pattern
    pub use crate::mutation::{
        Mutation, MutationState, provider_cache_key, provider_cache_key_simple, use_mutation,
        use_optimistic_mutation,
    };

    // Error types
    pub use crate::errors::{
        ApiError, ApiResult, DatabaseError, DatabaseResult, ProviderError, ProviderResult,
        UserError, UserResult,
    };
}
