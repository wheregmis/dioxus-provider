//! Provider struct and use_provider hook
//!
//! This module provides the simplified provider system that works with any async function.
//! Uses Dioxus Stores for fine-grained reactivity.

use std::marker::PhantomData;
use std::time::Duration;

use dioxus::core::Task;
use dioxus::core::current_scope_id;
use dioxus::prelude::*;
use dioxus_stores::{Store, use_store};

use crate::cache::{CacheGetOptions, Instant, ProviderCache};
use crate::callback::ProviderCallback;
use crate::global::get_global_runtime_handles;
use crate::refresh::RefreshRegistry;

/// The state of a provider
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub enum State {
    /// Provider has not been called yet
    Idle,
    /// Provider is currently fetching data
    Pending,
    /// Provider has successfully fetched data
    Ready,
    /// Provider encountered an error
    Error,
    /// Provider was reset/cancelled
    Reset,
}

/// Internal state for a provider, using Store for fine-grained reactivity
#[derive(Store, Clone, PartialEq)]
pub struct ProviderData<O: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> {
    /// The current state
    pub state: State,
    /// The current value (if successfully fetched)
    pub value: Option<O>,
    /// The current error (if fetch failed)
    pub error: Option<E>,
    /// Current cache key
    pub cache_key: Option<String>,
    /// Stale time for SWR behavior
    pub stale_time: Option<Duration>,
    /// Cache expiration time
    pub cache_expiration: Option<Duration>,
    /// Auto-refresh interval
    pub interval: Option<Duration>,
}

impl<O: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> Default
    for ProviderData<O, E>
{
    fn default() -> Self {
        Self {
            state: State::Idle,
            value: None,
            error: None,
            cache_key: None,
            stale_time: None,
            cache_expiration: None,
            interval: None,
        }
    }
}

/// A provider that fetches and caches async data.
///
/// Uses Dioxus Store for fine-grained reactivity - components only re-render
/// when the specific fields they access change.
///
/// Created by calling `use_provider(async_fn)`. Configure with builder methods
/// like `.stale_time()` and `.cache_expiration()`, then call `.call(args)` to fetch.
///
/// # Example
///
/// ```rust,ignore
/// async fn fetch_user(id: u32) -> Result<User, Error> {
///     // fetch user from API
/// }
///
/// let mut user = use_provider(fetch_user)
///     .stale_time(Duration::from_secs(60))
///     .cache_expiration(Duration::from_secs(300));
///
/// // Trigger fetch
/// user.call(123);
///
/// // Access value with fine-grained reactivity
/// if user.data().state() == State::Ready {
///     let value = user.data().value();  // Only subscribes to value changes
/// }
/// ```
#[derive(PartialEq)]
pub struct Provider<I: 'static, O: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> {
    /// Store containing the provider state - provides fine-grained reactivity
    pub(crate) store: Store<ProviderData<O, E>>,
    /// The current task (if fetching)
    pub(crate) task: Signal<Option<Task>>,
    /// The callback to execute
    pub(crate) callback: Signal<Box<dyn Fn(I) -> String + 'static>>,
    /// Function to generate cache keys
    #[allow(clippy::type_complexity)]
    pub(crate) cache_key_fn: Signal<Box<dyn Fn(&I) -> String + 'static>>,
    /// Cache reference
    pub(crate) cache: Signal<ProviderCache>,
    /// Refresh registry reference
    pub(crate) refresh_registry: Signal<RefreshRegistry>,
    /// Phantom data for input type
    pub(crate) _phantom: PhantomData<I>,
}

// Provider is Copy because all fields are Copy (Store and Signal are Copy)
impl<I: 'static, O: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> Copy
    for Provider<I, O, E>
{
}

impl<I: 'static, O: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> Clone
    for Provider<I, O, E>
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<I: 'static, O: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> Provider<I, O, E> {
    /// Get the underlying store for fine-grained access
    ///
    /// Use this to access individual fields with fine-grained reactivity:
    /// ```rust,ignore
    /// let state = provider.data().state();  // Only re-renders on state change
    /// let value = provider.data().value();  // Only re-renders on value change
    /// ```
    pub fn data(&self) -> Store<ProviderData<O, E>> {
        self.store
    }

    /// Set the stale time for SWR (stale-while-revalidate) behavior.
    ///
    /// Data older than this duration will be considered stale and will trigger
    /// a background revalidation while still serving the stale data.
    pub fn stale_time(self, duration: Duration) -> Self {
        if self.store.stale_time().read().as_ref() != Some(&duration) {
            self.store.stale_time().set(Some(duration));
        }
        self
    }

    /// Set the cache expiration time.
    ///
    /// Data older than this duration will be removed from cache entirely,
    /// forcing a fresh fetch on the next access.
    pub fn cache_expiration(self, duration: Duration) -> Self {
        if self.store.cache_expiration().read().as_ref() != Some(&duration) {
            self.store.cache_expiration().set(Some(duration));
        }
        self
    }

    /// Set the auto-refresh interval.
    ///
    /// The provider will automatically refetch data at this interval.
    pub fn interval(self, duration: Duration) -> Self {
        if self.store.interval().read().as_ref() != Some(&duration) {
            self.store.interval().set(Some(duration));
        }
        self
    }

    /// Get the current value if available.
    ///
    /// Returns `Some(Ok(value))` if data was successfully fetched,
    /// `Some(Err(error))` if fetch failed, or `None` if not yet fetched.
    pub fn value(&self) -> Option<Result<O, E>> {
        let state = self.store.state().cloned();
        match state {
            State::Ready => self.store.value().cloned().map(Ok),
            State::Error => self.store.error().cloned().map(Err),
            _ => None,
        }
    }

    /// Get the current data if successfully fetched.
    pub fn get_data(&self) -> Option<O> {
        if self.store.state().cloned() == State::Ready {
            self.store.value().cloned()
        } else {
            None
        }
    }

    /// Get the current error if fetch failed.
    pub fn error(&self) -> Option<E> {
        if self.store.state().cloned() == State::Error {
            self.store.error().cloned()
        } else {
            None
        }
    }

    /// Check if the provider is currently fetching.
    pub fn pending(&self) -> bool {
        self.store.state().cloned() == State::Pending
    }

    /// Check if the provider is idle (not yet called).
    pub fn idle(&self) -> bool {
        self.store.state().cloned() == State::Idle
    }

    /// Check if the provider has data ready.
    pub fn ready(&self) -> bool {
        self.store.state().cloned() == State::Ready
    }

    /// Check if the provider has errored.
    pub fn errored(&self) -> bool {
        self.store.state().cloned() == State::Error
    }

    /// Get the current state.
    pub fn state(&self) -> State {
        self.store.state().cloned()
    }

    /// Reset the provider, clearing value and error.
    pub fn reset(&mut self) {
        if let Some(task) = self.task.write().take() {
            task.cancel();
        }
        self.store.state().set(State::Reset);
        self.store.value().set(None);
        self.store.error().set(None);
        self.store.cache_key().set(None);
    }

    /// Cancel any pending fetch.
    pub fn cancel(&mut self) {
        if let Some(task) = self.task.write().take() {
            task.cancel();
        }
        if self.store.state().cloned() == State::Pending {
            self.store.state().set(State::Reset);
        }
    }

    /// Invalidate the cache for this provider's current key.
    pub fn invalidate(&self) {
        if let Some(key) = self.store.cache_key().cloned() {
            self.cache.read().invalidate(&key);
            self.refresh_registry.read().trigger_refresh(&key);
        }
    }

    /// Get the current cache key (if set).
    pub fn cache_key(&self) -> Option<String> {
        self.store.cache_key().cloned()
    }
}

impl<I: 'static, O: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> std::fmt::Debug
    for Provider<I, O, E>
where
    O: std::fmt::Debug,
    E: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Provider")
            .field("state", &self.store.state().cloned())
            .field("value", &self.store.value().cloned())
            .field("error", &self.store.error().cloned())
            .finish()
    }
}

// Implement call methods for different arities

impl<O: Clone + Send + Sync + PartialEq + 'static, E: Clone + Send + Sync + PartialEq + 'static>
    Provider<(), O, E>
{
    /// Call the provider with no arguments.
    pub fn call(&mut self) {
        let cache_key = (self.cache_key_fn.read())(&());
        self.store.cache_key().set(Some(cache_key.clone()));
        let should_fetch = check_cache_and_set_state(
            self.store,
            self.task,
            &self.cache.read(),
            &self.refresh_registry.read(),
            &cache_key,
        );
        if should_fetch {
            // Trigger the actual fetch
            (self.callback.read())(());
        }
    }
}

impl<
    A: Clone + 'static,
    O: Clone + Send + Sync + PartialEq + 'static,
    E: Clone + Send + Sync + PartialEq + 'static,
> Provider<(A,), O, E>
{
    /// Call the provider with one argument.
    pub fn call(&mut self, a: A) {
        let input = (a.clone(),);
        let cache_key = (self.cache_key_fn.read())(&input);
        self.store.cache_key().set(Some(cache_key.clone()));
        let should_fetch = check_cache_and_set_state(
            self.store,
            self.task,
            &self.cache.read(),
            &self.refresh_registry.read(),
            &cache_key,
        );
        if should_fetch {
            (self.callback.read())(input);
        }
    }
}

impl<
    A: Clone + 'static,
    B: Clone + 'static,
    O: Clone + Send + Sync + PartialEq + 'static,
    E: Clone + Send + Sync + PartialEq + 'static,
> Provider<(A, B), O, E>
{
    /// Call the provider with two arguments.
    pub fn call(&mut self, a: A, b: B) {
        let input = (a.clone(), b.clone());
        let cache_key = (self.cache_key_fn.read())(&input);
        self.store.cache_key().set(Some(cache_key.clone()));
        let should_fetch = check_cache_and_set_state(
            self.store,
            self.task,
            &self.cache.read(),
            &self.refresh_registry.read(),
            &cache_key,
        );
        if should_fetch {
            (self.callback.read())(input);
        }
    }
}

impl<
    A: Clone + 'static,
    B: Clone + 'static,
    C: Clone + 'static,
    O: Clone + Send + Sync + PartialEq + 'static,
    E: Clone + Send + Sync + PartialEq + 'static,
> Provider<(A, B, C), O, E>
{
    /// Call the provider with three arguments.
    pub fn call(&mut self, a: A, b: B, c: C) {
        let input = (a.clone(), b.clone(), c.clone());
        let cache_key = (self.cache_key_fn.read())(&input);
        self.store.cache_key().set(Some(cache_key.clone()));
        let should_fetch = check_cache_and_set_state(
            self.store,
            self.task,
            &self.cache.read(),
            &self.refresh_registry.read(),
            &cache_key,
        );
        if should_fetch {
            (self.callback.read())(input);
        }
    }
}

impl<
    A: Clone + 'static,
    B: Clone + 'static,
    C: Clone + 'static,
    D: Clone + 'static,
    O: Clone + Send + Sync + PartialEq + 'static,
    E: Clone + Send + Sync + PartialEq + 'static,
> Provider<(A, B, C, D), O, E>
{
    /// Call the provider with four arguments.
    pub fn call(&mut self, a: A, b: B, c: C, d: D) {
        let input = (a.clone(), b.clone(), c.clone(), d.clone());
        let cache_key = (self.cache_key_fn.read())(&input);
        self.store.cache_key().set(Some(cache_key.clone()));
        let should_fetch = check_cache_and_set_state(
            self.store,
            self.task,
            &self.cache.read(),
            &self.refresh_registry.read(),
            &cache_key,
        );
        if should_fetch {
            (self.callback.read())(input);
        }
    }
}

/// Internal function to check cache and set initial state.
/// Returns true if a fetch should proceed, false if cache hit was sufficient.
fn check_cache_and_set_state<
    O: Clone + Send + Sync + PartialEq + 'static,
    E: Clone + Send + Sync + PartialEq + 'static,
>(
    store: Store<ProviderData<O, E>>,
    mut task_signal: Signal<Option<Task>>,
    cache: &ProviderCache,
    refresh_registry: &RefreshRegistry,
    cache_key: &str,
) -> bool {
    // Cancel any existing task
    if let Some(task) = task_signal.write().take() {
        task.cancel();
    }

    // Check cache first
    let stale_time = store.stale_time().cloned();
    let cache_expiration = store.cache_expiration().cloned();

    let cache_options = CacheGetOptions {
        expiration: cache_expiration,
        stale_time,
        check_staleness: stale_time.is_some(),
    };

    if let Some(result) = cache.get_with_options::<Result<O, E>>(cache_key, cache_options.clone()) {
        match result.data {
            Ok(data) => {
                store.value().set(Some(data));
                store.error().set(None);
                store.state().set(State::Ready);

                // If stale, trigger background revalidation
                if result.is_stale {
                    crate::debug_log!(
                        "📦 [CACHE-HIT-STALE] Serving stale data for key: {}, triggering revalidation",
                        cache_key
                    );
                    refresh_registry.trigger_refresh(cache_key);
                    return true; // Proceed with background fetch
                } else {
                    crate::debug_log!("📦 [CACHE-HIT] Serving fresh data for key: {}", cache_key);
                }
                return false; // No fetch needed
            }
            Err(e) => {
                // Cached error - still return it but maybe refetch
                store.error().set(Some(e));
                store.value().set(None);
                store.state().set(State::Error);
                return false;
            }
        }
    }

    // No cache hit, need to fetch
    store.state().set(State::Pending);
    crate::debug_log!("🔄 [FETCH] Starting fetch for key: {}", cache_key);
    true
}

/// Create a provider from an async function.
///
/// This is the main entry point for the simplified provider system.
/// Works with any async function that returns `Result<T, E>`.
///
/// Uses Dioxus Store for fine-grained reactivity - components only re-render
/// when the specific fields they access change.
///
/// # Example
///
/// ```rust,ignore
/// async fn fetch_user(id: u32) -> Result<User, Error> {
///     // fetch user from API
/// }
///
/// let mut user = use_provider(fetch_user)
///     .stale_time(Duration::from_secs(60));
///
/// user.call(123);
///
/// // Fine-grained reactivity - only subscribes to state changes
/// if user.data().state().cloned() == State::Ready {
///     // This only re-renders when value changes
///     let value = user.data().value().cloned();
/// }
/// ```
pub fn use_provider<F, M, E, O>(user_fn: F) -> Provider<F::Input, O, E>
where
    E: Clone + Send + Sync + PartialEq + 'static,
    F: ProviderCallback<M, E, Output = O> + 'static,
    M: 'static,
    F::Input: Clone + std::hash::Hash + 'static,
    O: Clone + Send + Sync + PartialEq + 'static,
{
    // Use store for fine-grained reactivity
    let store: Store<ProviderData<O, E>> = use_store(ProviderData::default);
    let task: Signal<Option<Task>> = use_signal(|| None);

    // Get global cache and refresh registry
    let (cache_val, refresh_registry_val) = get_global_runtime_handles()
        .map(|h| (h.cache, h.refresh_registry))
        .unwrap_or_else(|_| (ProviderCache::new(), RefreshRegistry::new()));

    // Store in signals for Copy
    let cache: Signal<ProviderCache> = use_signal(|| cache_val.clone());
    let refresh_registry: Signal<RefreshRegistry> = use_signal(|| refresh_registry_val.clone());

    // Create callback that captures the user function
    let user_fn_clone = user_fn.clone();
    let cache_clone = cache_val.clone();
    let refresh_registry_clone = refresh_registry_val.clone();

    let callback: Signal<Box<dyn Fn(F::Input) -> String + 'static>> =
        use_signal(move || -> Box<dyn Fn(F::Input) -> String + 'static> {
            let user_fn = user_fn_clone.clone();
            let cache = cache_clone.clone();
            let refresh_registry = refresh_registry_clone.clone();

            Box::new(move |input: F::Input| {
                let cache_key = user_fn.cache_key(&input);
                let user_fn = user_fn.clone();
                let cache = cache.clone();
                let refresh_registry = refresh_registry.clone();
                let cache_key_clone = cache_key.clone();

                // Spawn the actual fetch
                spawn(async move {
                    let start_time = Instant::now();
                    let result = user_fn.call(input).await;

                    match &result {
                        Ok(data) => {
                            store.error().set(None);
                            store.value().set(Some(data.clone()));
                            store.state().set(State::Ready);
                        }
                        Err(e) => {
                            store.error().set(Some(e.clone()));
                            store.value().set(None);
                            store.state().set(State::Error);
                        }
                    }

                    // Store in cache, preventing race conditions
                    if cache.set_if_not_updated_since(cache_key_clone.clone(), result, start_time) {
                        // Trigger refresh for any subscribers ONLY if we actually updated the cache
                        refresh_registry.trigger_refresh(&cache_key_clone);
                    }
                });

                cache_key
            })
        });

    // Create cache key function
    let cache_key_fn: Signal<Box<dyn Fn(&F::Input) -> String + 'static>> =
        use_signal(move || -> Box<dyn Fn(&F::Input) -> String + 'static> {
            let user_fn = user_fn.clone();
            Box::new(move |input: &F::Input| user_fn.cache_key(input))
        });

    // Subscribe to refresh registry to react to external updates (e.g. optimistic mutations)
    // We use the current scope ID to schedule updates directly
    let scope_id = current_scope_id();

    use_effect(move || {
        if let Some(key) = store.cache_key().read().clone() {
            let reg = refresh_registry.read();
            reg.subscribe(&key, scope_id);
        }
    });

    // Sync with global cache on every render (if triggered by refresh_trigger)
    // This ensures we pick up optimistic updates immediately.
    // We use a separate block to ensure all read locks are dropped before we try to write.
    let update_to_apply = {
        let cache_key_signal = store.cache_key();
        let key_read = cache_key_signal.read();
        if let Some(key) = key_read.as_ref() {
            let cache_read = cache.read();
            if let Some(Ok(data)) = cache_read.get::<Result<O, E>>(key) {
                // Check if we need to update
                let value_signal = store.value();
                let value_read = value_signal.read();
                if value_read.as_ref() != Some(&data) {
                    Some(data)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    };

    if let Some(data) = update_to_apply {
        crate::debug_log!("🔄 [SYNC] Syncing provider with global cache",);
        store.value().set(Some(data));
        store.state().set(State::Ready);
        store.error().set(None);
    }

    Provider {
        store,
        task,
        callback,
        cache_key_fn,
        cache,
        refresh_registry,
        _phantom: PhantomData,
    }
}
