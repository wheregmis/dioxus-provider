#![doc = include_str!("../README.md")]

// Core modules
pub mod cache;
pub mod callback;
pub mod errors;
pub mod global;
pub mod hooks;
pub mod injection;
mod log_utils;
pub mod mutation;
pub mod mutation_new;
pub mod param_utils;
pub mod platform;
pub mod provider;
pub mod refresh;
mod runtime;
mod state;
pub mod types;

// Re-export commonly used items at crate root for convenience
pub use global::ProviderConfig;
pub use global::init;

pub mod prelude {
    //! The prelude exports all the most common types and functions for using dioxus-provider.

    // The main provider trait and the macro (legacy API)
    pub use crate::hooks::Provider;
    pub use dioxus_provider_macros::{mutation, provider};

    // The core hook for using providers (legacy API)
    pub use crate::hooks::use_provider as use_provider_legacy;

    // NEW: Simplified provider API (works with any async fn, uses Stores for fine-grained reactivity)
    pub use crate::callback::ProviderCallback;
    pub use crate::provider::{use_provider, Provider as ProviderHandle, ProviderState, ProviderData};
    pub use crate::mutation_new::{use_mutation as use_mutation_new, Mutation as MutationHandle, MutationState as MutationStateNew, MutationData};

    // Hooks for manual cache management
    pub use crate::hooks::use_clear_provider_cache;
    pub use crate::hooks::use_invalidate_provider;
    pub use crate::hooks::use_provider_cache;

    // The async state enum, needed for matching
    pub use crate::state::State;

    // Global initialization
    pub use crate::global::{ProviderConfig, init};

    // Dependency Injection
    pub use crate::injection::{ensure_dependency, inject, register_dependency};

    // Mutation system (legacy API)
    pub use crate::mutation::{
        Mutation, MutationContext, MutationState, provider_cache_key, use_mutation as use_mutation_legacy,
    };

    // Error types
    pub use crate::errors::ProviderError;
}
