#![doc = include_str!("../README.md")]

// Core modules
pub mod cache;
pub mod callback;
pub mod errors;
pub mod global;
pub mod injection;
mod log_utils;
pub mod mutation;
pub mod platform;
pub mod provider;
pub mod refresh;
mod runtime;
pub mod types;

// Re-export commonly used items at crate root for convenience
pub use global::ProviderConfig;
pub use global::init;

pub mod prelude {
    //! The prelude exports all the most common types and functions for using dioxus-provider.

    // Simplified provider API (works with any async fn, uses Stores for fine-grained reactivity)
    pub use crate::callback::ProviderCallback;
    pub use crate::provider::{use_provider, Provider, ProviderData, State};
    pub use crate::mutation::{use_mutation, Mutation as MutationHandle, MutationState, MutationData};

    // Global initialization
    pub use crate::global::{ProviderConfig, init};

    // Dependency Injection
    pub use crate::injection::{ensure_dependency, inject, register_dependency};

    // Error types
    pub use crate::errors::ProviderError;
}
