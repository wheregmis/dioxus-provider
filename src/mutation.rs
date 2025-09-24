//! # Mutation System for Dioxus Provider
//!
//! This module provides mutation capabilities for modifying data and keeping caches in sync.
//! It integrates seamlessly with the provider system for automatic cache invalidation and
//! optimistic updates.
//!
//! ## Features
//!
//! - **Mutation Providers**: Define mutations with the `#[mutation]` macro
//! - **Optimistic Updates**: Immediate UI updates with rollback on failure
//! - **Automatic Cache Invalidation**: Invalidate related providers automatically
//! - **Mutation State**: Track loading, success, and error states
//! - **Rollback Support**: Automatic rollback of optimistic updates on failure

use dioxus::prelude::*;
use std::future::Future;
use tracing::debug;

use crate::{
    global::{get_global_cache, get_global_refresh_registry},
    hooks::Provider,
    types::ProviderParamBounds,
};

/// Represents the state of a mutation operation
#[derive(Clone, PartialEq)]
pub enum MutationState<T, E> {
    /// The mutation is idle (not running)
    Idle,
    /// The mutation is currently loading
    Loading,
    /// The mutation completed successfully
    Success(T),
    /// The mutation failed with an error
    Error(E),
}

impl<T, E> MutationState<T, E> {
    /// Returns true if the mutation is idle
    pub fn is_idle(&self) -> bool {
        matches!(self, MutationState::Idle)
    }

    /// Returns true if the mutation is currently loading
    pub fn is_loading(&self) -> bool {
        matches!(self, MutationState::Loading)
    }

    /// Returns true if the mutation completed successfully
    pub fn is_success(&self) -> bool {
        matches!(self, MutationState::Success(_))
    }

    /// Returns true if the mutation failed
    pub fn is_error(&self) -> bool {
        matches!(self, MutationState::Error(_))
    }

    /// Returns the success data if available
    pub fn data(&self) -> Option<&T> {
        match self {
            MutationState::Success(data) => Some(data),
            _ => None,
        }
    }

    /// Returns the error if available
    pub fn error(&self) -> Option<&E> {
        match self {
            MutationState::Error(error) => Some(error),
            _ => None,
        }
    }
}

/// Trait for defining mutations - operations that modify data
///
/// Mutations are similar to providers but are designed for data modification operations.
/// They typically involve server requests to create, update, or delete data.
///
/// ## Usage
/// Prefer using the `#[mutation]` macro to define mutations. Manual trait implementations are for advanced use only.
///
/// ## Example
///
/// ```rust,no_run
/// use dioxus_provider::prelude::*;
///
/// #[mutation(invalidates = [fetch_user, fetch_users])]
/// async fn update_user(user_id: u32, data: UserData) -> Result<User, String> {
///     // Make API call to update user
///     api_client.update_user(user_id, data).await
/// }
/// ```
pub trait Mutation<Input = ()>: Clone + PartialEq + 'static
where
    Input: Clone + PartialEq + 'static,
{
    /// The type of data returned on successful mutation
    type Output: Clone + PartialEq + Send + Sync + 'static;
    /// The type of error returned on mutation failure
    type Error: Clone + PartialEq + Send + Sync + 'static;

    /// Execute the mutation with the given input
    fn mutate(&self, input: Input) -> impl Future<Output = Result<Self::Output, Self::Error>>;

    /// Get a unique identifier for this mutation type
    fn id(&self) -> String {
        std::any::type_name::<Self>().to_string()
    }

    /// Get list of provider cache keys that should be invalidated after successful mutation
    /// Override this to specify which providers should be refreshed after mutation
    fn invalidates(&self) -> Vec<String> {
        Vec::new()
    }

    /// Provide optimistic cache updates for immediate UI feedback
    /// Returns a list of (cache_key, optimistic_result) pairs to update the cache with
    /// This allows the UI to update immediately with the expected result
    /// The Result should contain the optimistic success value
    fn optimistic_updates(
        &self,
        _input: &Input,
    ) -> Vec<(String, Result<Self::Output, Self::Error>)> {
        Vec::new()
    }
}

/// Type alias for the return type of mutation hooks
pub type MutationHookResult<M, Input, F> = (
    Signal<MutationState<<M as Mutation<Input>>::Output, <M as Mutation<Input>>::Error>>,
    F,
);

/// Hook to create a mutation that can be triggered manually
///
/// Returns a tuple containing:
/// 1. A signal with the current mutation state
/// 2. A function to trigger the mutation
///
/// ## Example
///
/// ```rust,no_run
/// use dioxus::prelude::*;
/// use dioxus_provider::prelude::*;
///
/// #[component]
/// fn UpdateUserForm(user_id: u32) -> Element {
///     let (mutation_state, mutate) = use_mutation(update_user());
///     
///     let handle_submit = move |data: UserData| {
///         mutate(user_id, data);
///     };
///     
///     rsx! {
///         form {
///             button {
///                 disabled: mutation_state.read().is_loading(),
///                 onclick: move |_| handle_submit(get_form_data()),
///                 "Update User"
///             }
///             match &*mutation_state.read() {
///                 MutationState::Loading => rsx! { div { "Updating..." } },
///                 MutationState::Success(_) => rsx! { div { "Updated successfully!" } },
///                 MutationState::Error(err) => rsx! { div { "Error: {err}" } },
///                 MutationState::Idle => rsx! { div {} },
///             }
///         }
///     }
/// }
/// ```
pub fn use_mutation<M, Input>(mutation: M) -> MutationHookResult<M, Input, impl Fn(Input) + Clone>
where
    M: Mutation<Input> + Send + Sync + 'static,
    Input: Clone + PartialEq + Send + Sync + 'static,
{
    let state = use_signal(|| MutationState::Idle);
    let cache = get_global_cache();
    let refresh_registry = get_global_refresh_registry();

    let mutate_fn = {
        let mutation = mutation.clone();
        let cache = cache.expect("Global providers not initialized").clone();
        let refresh_registry = refresh_registry
            .expect("Global providers not initialized")
            .clone();
        let mut state = state;

        move |input: Input| {
            let mutation = mutation.clone();
            let cache = cache.clone();
            let refresh_registry = refresh_registry.clone();
            let input = input.clone();

            spawn(async move {
                state.set(MutationState::Loading);

                debug!("🔄 [MUTATION] Starting mutation: {}", mutation.id());

                match mutation.mutate(input).await {
                    Ok(result) => {
                        debug!("✅ [MUTATION] Mutation succeeded: {}", mutation.id());

                        // Invalidate specified cache entries
                        for cache_key in mutation.invalidates() {
                            debug!("🗑️ [MUTATION] Invalidating cache key: {}", cache_key);
                            cache.invalidate(&cache_key);
                            refresh_registry.trigger_refresh(&cache_key);
                        }

                        state.set(MutationState::Success(result));
                    }
                    Err(error) => {
                        debug!("❌ [MUTATION] Mutation failed: {}", mutation.id());
                        state.set(MutationState::Error(error));
                    }
                }
            });
        }
    };

    (state, mutate_fn)
}

/// Hook to create a mutation with optimistic invalidation
///
/// This variant optimistically invalidates cache entries immediately when the mutation
/// is triggered, providing instant feedback while the mutation is in progress.
///
/// ## Example
///
/// ```rust,no_run
/// use dioxus::prelude::*;
/// use dioxus_provider::prelude::*;
///
/// #[component]
/// fn TodoItem(todo_id: u32) -> Element {
///     let (mutation_state, mutate) = use_optimistic_mutation(toggle_todo());
///     
///     rsx! {
///         div {
///             button {
///                 onclick: move |_| mutate(todo_id),
///                 "Toggle Todo"
///             }
///             match &*mutation_state.read() {
///                 MutationState::Loading => rsx! { span { "Saving..." } },
///                 MutationState::Error(err) => rsx! { span { "Error: {err}" } },
///                 _ => rsx! { span {} },
///             }
///         }
///     }
/// }
/// ```
pub fn use_optimistic_mutation<M, Input>(
    mutation: M,
) -> MutationHookResult<M, Input, impl Fn(Input) + Clone>
where
    M: Mutation<Input> + Send + Sync + 'static,
    Input: Clone + PartialEq + Send + Sync + 'static,
{
    let state = use_signal(|| MutationState::Idle);
    let cache = get_global_cache();
    let refresh_registry = get_global_refresh_registry();

    let mutate_fn = {
        let mutation = mutation.clone();
        let cache = cache.expect("Global providers not initialized").clone();
        let refresh_registry = refresh_registry
            .expect("Global providers not initialized")
            .clone();
        let mut state = state;

        move |input: Input| {
            let mutation = mutation.clone();
            let cache = cache.clone();
            let refresh_registry = refresh_registry.clone();
            let input = input.clone();

            spawn(async move {
                // Apply optimistic updates for immediate feedback
                let optimistic_updates = mutation.optimistic_updates(&input);
                if !optimistic_updates.is_empty() {
                    debug!(
                        "⚡ [OPTIMISTIC] Optimistically updating {} cache entries",
                        optimistic_updates.len()
                    );
                    for (cache_key, optimistic_result) in &optimistic_updates {
                        cache.set(cache_key.clone(), optimistic_result.clone());
                        refresh_registry.trigger_refresh(cache_key);
                    }
                }

                state.set(MutationState::Loading);

                debug!(
                    "🔄 [MUTATION] Starting optimistic mutation: {}",
                    mutation.id()
                );

                match mutation.mutate(input).await {
                    Ok(result) => {
                        debug!(
                            "✅ [MUTATION] Optimistic mutation succeeded: {}",
                            mutation.id()
                        );

                        // Invalidate specified cache entries (ensuring fresh data)
                        for cache_key in mutation.invalidates() {
                            debug!("🗑️ [MUTATION] Invalidating cache key: {}", cache_key);
                            cache.invalidate(&cache_key);
                            refresh_registry.trigger_refresh(&cache_key);
                        }

                        state.set(MutationState::Success(result));
                    }
                    Err(error) => {
                        debug!(
                            "❌ [MUTATION] Optimistic mutation failed: {}",
                            mutation.id()
                        );

                        // Rollback optimistic updates by invalidating cache to trigger refetch
                        for (cache_key, _) in &optimistic_updates {
                            debug!(
                                "🔄 [ROLLBACK] Rolling back optimistic update for cache key: {}",
                                cache_key
                            );
                            cache.invalidate(cache_key);
                            refresh_registry.trigger_refresh(cache_key);
                        }

                        state.set(MutationState::Error(error));
                    }
                }
            });
        }
    };

    (state, mutate_fn)
}

/// Helper function to create cache keys for providers with parameters
pub fn provider_cache_key<P, Param>(provider: P, param: Param) -> String
where
    P: Provider<Param>,
    Param: ProviderParamBounds,
{
    provider.id(&param)
}

/// Helper function to create cache keys for providers without parameters
pub fn provider_cache_key_simple<P>(provider: P) -> String
where
    P: Provider<()>,
{
    provider.id(&())
}
