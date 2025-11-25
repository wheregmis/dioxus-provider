//! Mutation struct and use_mutation hook
//!
//! This module provides mutations - async operations that modify data and can
//! invalidate provider caches. Uses Dioxus Stores for fine-grained reactivity.

use std::marker::PhantomData;

use dioxus::prelude::*;
use dioxus::core::Task;
use dioxus_stores::{Store, use_store};

use crate::cache::ProviderCache;
use crate::callback::ProviderCallback;
use crate::global::get_global_runtime_handles;
use crate::refresh::RefreshRegistry;

/// The state of a mutation
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub enum MutationState {
    /// Mutation has not been called yet
    Idle,
    /// Mutation is currently executing
    Pending,
    /// Mutation completed successfully
    Success,
    /// Mutation encountered an error
    Errored,
    /// Mutation was reset/cancelled
    Reset,
}

/// Internal state for a mutation, using Store for fine-grained reactivity
#[derive(Store, Clone, PartialEq)]
pub struct MutationData<O: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> {
    /// The current state
    pub state: MutationState,
    /// The current value (if mutation succeeded)
    pub value: Option<O>,
    /// The current error (if mutation failed)
    pub error: Option<E>,
}

impl<O: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> Default for MutationData<O, E> {
    fn default() -> Self {
        Self {
            state: MutationState::Idle,
            value: None,
            error: None,
        }
    }
}

/// A mutation that executes an async operation and can invalidate provider caches.
///
/// Uses Dioxus Store for fine-grained reactivity - components only re-render
/// when the specific fields they access change.
///
/// Created by calling `use_mutation(async_fn)`. Configure with builder methods
/// like `.invalidates()` and `.optimistic()`.
///
/// # Example
///
/// ```rust,ignore
/// async fn update_user(id: u32, name: String) -> Result<User, Error> {
///     // update user in API
/// }
///
/// let mut update = use_mutation(update_user)
///     .invalidates(fetch_user);
///
/// // Execute mutation
/// update.call(123, "New Name".into());
///
/// // Fine-grained reactivity
/// if update.data().state().cloned() == MutationState::Pending {
///     // show loading
/// }
/// ```
pub struct Mutation<I: 'static, O: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> {
    /// Store containing the mutation state - provides fine-grained reactivity
    pub(crate) store: Store<MutationData<O, E>>,
    /// The current task (if executing)
    pub(crate) task: Signal<Option<Task>>,
    /// The callback to execute
    pub(crate) callback: Signal<Box<dyn Fn(I) + 'static>>,
    /// Cache keys to invalidate after mutation
    pub(crate) invalidation_keys: Signal<Vec<String>>,
    /// Cache reference
    pub(crate) cache: Signal<ProviderCache>,
    /// Refresh registry reference
    pub(crate) refresh_registry: Signal<RefreshRegistry>,
    /// Phantom data for input type
    pub(crate) _phantom: PhantomData<I>,
}

// Mutation is Copy because all fields are Copy (Store and Signal are Copy)
impl<I: 'static, O: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> Copy for Mutation<I, O, E> {}

impl<I: 'static, O: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> Clone for Mutation<I, O, E> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<I: 'static, O: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> Mutation<I, O, E> {
    /// Get the underlying store for fine-grained access
    ///
    /// Use this to access individual fields with fine-grained reactivity:
    /// ```rust,ignore
    /// let state = mutation.data().state();  // Only re-renders on state change
    /// let value = mutation.data().value();  // Only re-renders on value change
    /// ```
    pub fn data(&self) -> Store<MutationData<O, E>> {
        self.store
    }

    /// Add a provider to invalidate after this mutation completes.
    ///
    /// You can chain multiple `.invalidates()` calls to invalidate multiple providers.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let mut update = use_mutation(update_user)
    ///     .invalidates(fetch_user)
    ///     .invalidates(fetch_user_list);
    /// ```
    pub fn invalidates<F, M, E2>(mut self, provider_fn: F) -> Self
    where
        F: ProviderCallback<M, E2> + 'static,
        M: 'static,
        E2: 'static,
        F::Input: Default,
    {
        // Generate cache key for the provider with default input
        // This works for providers that don't take parameters
        let cache_key = provider_fn.cache_key(&F::Input::default());
        self.invalidation_keys.write().push(cache_key);
        self
    }

    /// Add a provider with specific input to invalidate after this mutation completes.
    ///
    /// Use this when you need to invalidate a provider with specific parameters.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let mut update = use_mutation(update_user)
    ///     .invalidates_with(fetch_user, (user_id,));
    /// ```
    pub fn invalidates_with<F, M, E2>(mut self, provider_fn: F, input: F::Input) -> Self
    where
        F: ProviderCallback<M, E2> + 'static,
        M: 'static,
        E2: 'static,
    {
        let cache_key = provider_fn.cache_key(&input);
        self.invalidation_keys.write().push(cache_key);
        self
    }

    /// Get the current value if mutation succeeded.
    pub fn value(&self) -> Option<Result<O, E>> {
        let state = self.store.state().cloned();
        match state {
            MutationState::Success => self.store.value().cloned().map(Ok),
            MutationState::Errored => self.store.error().cloned().map(Err),
            _ => None,
        }
    }

    /// Get the current data if mutation succeeded.
    pub fn get_data(&self) -> Option<O> {
        if self.store.state().cloned() == MutationState::Success {
            self.store.value().cloned()
        } else {
            None
        }
    }

    /// Get the current error if mutation failed.
    pub fn error(&self) -> Option<E> {
        if self.store.state().cloned() == MutationState::Errored {
            self.store.error().cloned()
        } else {
            None
        }
    }

    /// Check if the mutation is currently executing.
    pub fn pending(&self) -> bool {
        self.store.state().cloned() == MutationState::Pending
    }

    /// Check if the mutation is idle (not yet called).
    pub fn idle(&self) -> bool {
        self.store.state().cloned() == MutationState::Idle
    }

    /// Check if the mutation succeeded.
    pub fn success(&self) -> bool {
        self.store.state().cloned() == MutationState::Success
    }

    /// Check if the mutation errored.
    pub fn errored(&self) -> bool {
        self.store.state().cloned() == MutationState::Errored
    }

    /// Get the current state.
    pub fn state(&self) -> MutationState {
        self.store.state().cloned()
    }

    /// Reset the mutation, clearing value and error.
    pub fn reset(&mut self) {
        if let Some(task) = self.task.write().take() {
            task.cancel();
        }
        self.store.state().set(MutationState::Reset);
        self.store.value().set(None);
        self.store.error().set(None);
    }

    /// Cancel any pending mutation.
    pub fn cancel(&mut self) {
        if let Some(task) = self.task.write().take() {
            task.cancel();
        }
        if self.store.state().cloned() == MutationState::Pending {
            self.store.state().set(MutationState::Reset);
        }
    }
}

impl<I: 'static, O: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> std::fmt::Debug for Mutation<I, O, E>
where
    O: std::fmt::Debug,
    E: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mutation")
            .field("state", &self.store.state().cloned())
            .field("value", &self.store.value().cloned())
            .field("error", &self.store.error().cloned())
            .finish()
    }
}

// Implement call methods for different arities

impl<O: Clone + Send + Sync + PartialEq + 'static, E: Clone + PartialEq + 'static> Mutation<(), O, E> {
    /// Call the mutation with no arguments.
    pub fn call(&mut self) {
        self.store.state().set(MutationState::Pending);
        (self.callback.read())(());
    }
}

impl<A: Clone + 'static, O: Clone + Send + Sync + PartialEq + 'static, E: Clone + PartialEq + 'static>
    Mutation<(A,), O, E>
{
    /// Call the mutation with one argument.
    pub fn call(&mut self, a: A) {
        self.store.state().set(MutationState::Pending);
        (self.callback.read())((a,));
    }
}

impl<
        A: Clone + 'static,
        B: Clone + 'static,
        O: Clone + Send + Sync + PartialEq + 'static,
        E: Clone + PartialEq + 'static,
    > Mutation<(A, B), O, E>
{
    /// Call the mutation with two arguments.
    pub fn call(&mut self, a: A, b: B) {
        self.store.state().set(MutationState::Pending);
        (self.callback.read())((a, b));
    }
}

impl<
        A: Clone + 'static,
        B: Clone + 'static,
        C: Clone + 'static,
        O: Clone + Send + Sync + PartialEq + 'static,
        E: Clone + PartialEq + 'static,
    > Mutation<(A, B, C), O, E>
{
    /// Call the mutation with three arguments.
    pub fn call(&mut self, a: A, b: B, c: C) {
        self.store.state().set(MutationState::Pending);
        (self.callback.read())((a, b, c));
    }
}

impl<
        A: Clone + 'static,
        B: Clone + 'static,
        C: Clone + 'static,
        D: Clone + 'static,
        O: Clone + Send + Sync + PartialEq + 'static,
        E: Clone + PartialEq + 'static,
    > Mutation<(A, B, C, D), O, E>
{
    /// Call the mutation with four arguments.
    pub fn call(&mut self, a: A, b: B, c: C, d: D) {
        self.store.state().set(MutationState::Pending);
        (self.callback.read())((a, b, c, d));
    }
}

/// Create a mutation from an async function.
///
/// This is the main entry point for the simplified mutation system.
/// Works with any async function that returns `Result<T, E>`.
///
/// Uses Dioxus Store for fine-grained reactivity - components only re-render
/// when the specific fields they access change.
///
/// # Example
///
/// ```rust,ignore
/// async fn update_user(id: u32, name: String) -> Result<User, Error> {
///     // update user in API
/// }
///
/// let mut update = use_mutation(update_user)
///     .invalidates(fetch_user);
///
/// update.call(123, "New Name".into());
///
/// // Fine-grained reactivity
/// if update.data().state().cloned() == MutationState::Success {
///     let result = update.data().value().cloned();
/// }
/// ```
pub fn use_mutation<F, M, E, O>(user_fn: F) -> Mutation<F::Input, O, E>
where
    E: Clone + PartialEq + 'static,
    F: ProviderCallback<M, E, Output = O> + 'static,
    M: 'static,
    F::Input: Clone + std::hash::Hash + 'static,
    O: Clone + Send + Sync + PartialEq + 'static,
{
    // Use store for fine-grained reactivity
    let store: Store<MutationData<O, E>> = use_store(MutationData::default);
    let task_signal: Signal<Option<Task>> = use_signal(|| None);
    let invalidation_keys: Signal<Vec<String>> = use_signal(Vec::new);

    // Get global cache and refresh registry
    let (cache_val, refresh_registry_val) = get_global_runtime_handles()
        .map(|h| (h.cache, h.refresh_registry))
        .unwrap_or_else(|_| (ProviderCache::new(), RefreshRegistry::new()));

    // Store in signals for Copy
    let cache: Signal<ProviderCache> = use_signal(|| cache_val.clone());
    let refresh_registry: Signal<RefreshRegistry> = use_signal(|| refresh_registry_val.clone());

    // Create callback that captures the user function
    let cache_clone = cache_val.clone();
    let refresh_registry_clone = refresh_registry_val.clone();

    let callback: Signal<Box<dyn Fn(F::Input) + 'static>> =
        use_signal(move || -> Box<dyn Fn(F::Input) + 'static> {
            let user_fn = user_fn.clone();
            let cache = cache_clone.clone();
            let refresh_registry = refresh_registry_clone.clone();
            let invalidation_keys = invalidation_keys;

            Box::new(move |input: F::Input| {
                let user_fn = user_fn.clone();
                let cache = cache.clone();
                let refresh_registry = refresh_registry.clone();
                let invalidation_keys = invalidation_keys.read().clone();

                // Spawn the actual mutation
                spawn(async move {
                    let result = user_fn.call(input).await;

                    match result {
                        Ok(data) => {
                            store.error().set(None);
                            store.value().set(Some(data));
                            store.state().set(MutationState::Success);
                        }
                        Err(e) => {
                            store.error().set(Some(e));
                            store.value().set(None);
                            store.state().set(MutationState::Errored);
                        }
                    }

                    // Invalidate provider caches
                    for key in invalidation_keys.iter() {
                        cache.invalidate(key);
                        refresh_registry.trigger_refresh(key);
                        crate::debug_log!("🔄 [MUTATION] Invalidated provider cache: {}", key);
                    }
                });
            })
        });

    Mutation {
        store,
        task: task_signal,
        callback,
        invalidation_keys,
        cache,
        refresh_registry,
        _phantom: PhantomData,
    }
}
