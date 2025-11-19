//! # Provider Hooks
//!
//! This module provides hooks for working with providers in Dioxus applications.
//! It requires `dioxus_provider::global::init_global_providers()` to be called at application startup.
//!
//! ## Example
//!
//! ```rust
//! use dioxus::prelude::*;
//! use dioxus_provider::{prelude::*, global::init_global_providers};
//!
//! #[provider]
//! async fn fetch_user(id: u32) -> Result<String, String> {
//!     Ok(format!("User {}", id))
//! }
//!
//! #[component]
//! fn App() -> Element {
//!     let user = use_provider(fetch_user(), (1,));
//!     rsx! { div { "User: {user:?}" } }
//! }
//!
//! fn main() {
//!     init_global_providers();
//!     launch(App);
//! }
//! ```

use dioxus::{
    core::{ReactiveContext, SuspendedFuture},
    prelude::*,
};
use std::{fmt::Debug, future::Future, time::Duration};

use crate::{
    cache::ProviderCache,
    global::{get_global_runtime, get_global_runtime_handles},
    runtime::{ProviderRuntime, ProviderRuntimeHandles, request::handle_cache_miss},
};

use crate::param_utils::IntoProviderParam;
use crate::types::{ProviderErrorBounds, ProviderOutputBounds, ProviderParamBounds};

pub use crate::state::State;

/// A unified trait for defining providers - async operations that return data
///
/// This trait supports both simple providers (no parameters) and parameterized providers.
/// Use `Provider<()>` for simple providers and `Provider<ParamType>` for parameterized providers.
///
/// ## Features
///
/// - **Async Execution**: All providers are async by default
/// - **Configurable Caching**: Optional cache expiration times
/// - **Stale-While-Revalidate**: Serve stale data while revalidating in background
/// - **Auto-Refresh**: Optional automatic refresh at intervals
/// - **Auto-Dispose**: Automatic cleanup when providers are no longer used
///
/// ## Cross-Platform Compatibility
///
/// The Provider trait is designed to work across platforms using Dioxus's spawn system:
/// - Uses `dioxus::spawn` for async execution (no Send + Sync required for most types)
/// - Parameters may need Send + Sync if shared across contexts
/// - Output and Error types only need Clone since they stay within Dioxus context
///
/// ## Example
///
/// ```rust,no_run
/// use dioxus_provider::prelude::*;
/// use std::time::Duration;
///
/// #[provider(stale_time = "1m", cache_expiration = "5m")]
/// async fn data_provider() -> Result<String, String> {
///     // Fetch data from API
///     Ok("Hello, World!".to_string())
/// }
///
/// #[component]
/// fn Consumer() -> Element {
///     let data = use_provider(data_provider(), ());
///     // ...
/// }
/// ```
pub trait Provider<Param = ()>: Clone + PartialEq + 'static
where
    Param: ProviderParamBounds,
{
    /// The type of data returned on success
    type Output: ProviderOutputBounds;
    /// The type of error returned on failure
    type Error: ProviderErrorBounds;

    /// Execute the async operation
    ///
    /// This method performs the actual work of the provider, such as fetching data
    /// from an API, reading from a database, or computing a value.
    fn run(&self, param: Param) -> impl Future<Output = Result<Self::Output, Self::Error>>;

    /// Get a unique identifier for this provider instance with the given parameters
    ///
    /// This ID is used for caching and invalidation. The default implementation
    /// hashes the provider's type, parameter type, and parameter value to generate a unique ID.
    /// This ensures that different parameter types with the same value produce different keys.
    fn id(&self, param: &Param) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        // Hash provider type
        std::any::TypeId::of::<Self>().hash(&mut hasher);
        // Hash parameter type to prevent collisions between different types with same value
        std::any::TypeId::of::<Param>().hash(&mut hasher);
        // Hash parameter value
        param.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Get the interval duration for automatic refresh (None means no interval)
    ///
    /// When set, the provider will automatically refresh its data at the specified
    /// interval, even if no component is actively watching it.
    fn interval(&self) -> Option<Duration> {
        None
    }

    /// Get the cache expiration duration (None means no expiration)
    ///
    /// When set, cached data will be considered expired after this duration and
    /// will be removed from the cache, forcing a fresh fetch on the next access.
    fn cache_expiration(&self) -> Option<Duration> {
        None
    }

    /// Get the stale time duration for stale-while-revalidate behavior (None means no SWR)
    ///
    /// When set, data older than this duration will be considered stale and will
    /// trigger a background revalidation while still serving the stale data to the UI.
    fn stale_time(&self) -> Option<Duration> {
        None
    }
}

/// Extension trait to enable suspense support for provider signals
///
/// Allows you to call `.suspend()` on a `Signal<ProviderState<T, E>>`
/// inside a component. If the state is `Loading`, this will suspend
/// rendering and trigger Dioxus's SuspenseBoundary fallback.
///
/// Usage:
/// ```rust
/// let user = use_provider(fetch_user(), (1,)).suspend()?;
/// ```
pub trait SuspenseSignalExt<T, E> {
    /// Returns Ok(data) if ready, Err(RenderError::Suspended) if loading, or Ok(Err(error)) if error.
    fn suspend(&self) -> Result<Result<T, E>, RenderError>;
}

/// Error type for suspending rendering (compatible with Dioxus SuspenseBoundary)
#[derive(Debug, Clone, PartialEq)]
pub enum RenderError {
    Suspended(SuspendedFuture),
}

// Implement conversion so `?` works in components using Dioxus's RenderError
impl From<RenderError> for dioxus_core::RenderError {
    fn from(err: RenderError) -> Self {
        match err {
            RenderError::Suspended(fut) => dioxus_core::RenderError::Suspended(fut),
        }
    }
}

// Update SuspenseSignalExt to use ProviderState
impl<T: Clone + 'static, E: Clone + 'static> SuspenseSignalExt<T, E> for Signal<State<T, E>> {
    fn suspend(&self) -> Result<Result<T, E>, RenderError> {
        match &*self.read() {
            State::Loading { task } => Err(RenderError::Suspended(SuspendedFuture::new(*task))),
            State::Success(data) => Ok(Ok(data.clone())),
            State::Error(error) => Ok(Err(error.clone())),
        }
    }
}

fn runtime_handles_or_panic() -> ProviderRuntimeHandles {
    get_global_runtime_handles().unwrap_or_else(|_| {
        panic!(
            "Global providers not initialized. Call dioxus_provider::init() before using providers."
        )
    })
}

fn runtime_instance_or_panic() -> ProviderRuntime {
    get_global_runtime()
        .unwrap_or_else(|_| {
            panic!("Global providers not initialized. Call dioxus_provider::init() before using providers.")
        })
        .clone()
}

/// Get the provider cache - requires global providers to be initialized
fn get_provider_cache() -> ProviderCache {
    runtime_handles_or_panic().cache
}

/// Hook to access the provider cache for manual cache management
///
/// This hook provides direct access to the global provider cache for manual
/// invalidation, clearing, and other cache operations.
///
/// ## Global Providers Required
///
/// You must call `init_global_providers()` at application startup before using any provider hooks.
///
/// ## Setup
///
/// ```rust,no_run
/// use dioxus_provider::{prelude::*, global::init_global_providers};
///
/// fn main() {
///     init_global_providers();
///     dioxus::launch(App);
/// }
///
/// #[component]
/// fn App() -> Element {
///     rsx! {
///         MyComponent {}
///     }
/// }
/// ```
///
/// ## Example
///
/// ```rust,no_run
/// use dioxus::prelude::*;
/// use dioxus_provider::prelude::*;
///
/// #[component]
/// fn MyComponent() -> Element {
///     let cache = use_provider_cache();
///
///     // Manually invalidate a specific cache entry
///     cache.invalidate("my_provider_key");
///
///     rsx! {
///         div { "Cache operations example" }
///     }
/// }
/// ```
pub fn use_provider_cache() -> ProviderCache {
    get_provider_cache()
}

/// Hook to invalidate a specific provider cache entry
///
/// Returns a function that, when called, will invalidate the cache entry for the
/// specified provider and parameters, and trigger a refresh of all components
/// using that provider.
///
/// Requires global providers to be initialized with `init_global_providers()`.
///
/// ## Example
///
/// ```rust,no_run
/// use dioxus::prelude::*;
/// use dioxus_provider::prelude::*;
///
/// #[provider]
/// async fn user_provider(id: u32) -> Result<String, String> {
///     Ok(format!("User {}", id))
/// }
///
/// #[component]
/// fn MyComponent() -> Element {
///     let invalidate_user = use_invalidate_provider(user_provider(), 1);
///
///     rsx! {
///         button {
///             onclick: move |_| invalidate_user(),
///             "Refresh User Data"
///         }
///     }
/// }
/// ```
pub fn use_invalidate_provider<P, Param>(provider: P, param: Param) -> impl Fn() + Clone
where
    P: Provider<Param>,
    Param: ProviderParamBounds,
{
    let runtime = runtime_instance_or_panic();
    let runtime_handles = runtime.handles();
    let cache = runtime_handles.cache;
    let refresh_registry = runtime_handles.refresh_registry;
    let cache_key = provider.id(&param);

    move || {
        cache.invalidate(&cache_key);
        refresh_registry.trigger_refresh(&cache_key);
    }
}

/// Hook to clear the entire provider cache
///
/// Returns a function that, when called, will clear all cached provider data
/// and trigger a refresh of all providers currently in use.
///
/// Requires global providers to be initialized with `init_global_providers()`.
///
/// ## Example
///
/// ```rust,no_run
/// use dioxus::prelude::*;
/// use dioxus_provider::prelude::*;
///
/// #[component]
/// fn MyComponent() -> Element {
///     let clear_cache = use_clear_provider_cache();
///
///     rsx! {
///         button {
///             onclick: move |_| clear_cache(),
///             "Clear All Cache"
///         }
///     }
/// }
/// ```
pub fn use_clear_provider_cache() -> impl Fn() + Clone {
    let runtime = runtime_instance_or_panic();
    let runtime_handles = runtime.handles();
    let cache = runtime_handles.cache;
    let refresh_registry = runtime_handles.refresh_registry;

    move || {
        cache.clear();
        refresh_registry.clear_all();
    }
}

/// Unified trait for using providers with any parameter format
///
/// This trait provides a single, unified interface for using providers
/// regardless of their parameter format. It automatically handles:
/// - No parameters `()`
/// - Tuple parameters `(param,)`
/// - Direct parameters `param`
pub trait UseProvider<Args> {
    /// The type of data returned on success
    type Output: ProviderOutputBounds;
    /// The type of error returned on failure
    type Error: ProviderErrorBounds;

    /// Use the provider with the given arguments
    fn use_provider(self, args: Args) -> Signal<State<Self::Output, Self::Error>>;
}

/// Unified implementation for all providers using parameter normalization
///
/// This single implementation replaces all the previous repetitive implementations
/// by using the `IntoProviderParam` trait to normalize different parameter formats.
impl<P, Args> UseProvider<Args> for P
where
    P: Provider<Args::Param> + Send + Clone,
    Args: IntoProviderParam,
{
    type Output = P::Output;
    type Error = P::Error;

    fn use_provider(self, args: Args) -> Signal<State<Self::Output, Self::Error>> {
        let param = args.into_param();
        use_provider_core(self, param)
    }
}

/// Core provider implementation that handles all the common logic
fn use_provider_core<P, Param>(provider: P, param: Param) -> Signal<State<P::Output, P::Error>>
where
    P: Provider<Param> + Send + Clone,
    Param: ProviderParamBounds,
{
    let mut state = use_signal(|| State::Loading {
        task: spawn(async {}),
    });
    let runtime = runtime_instance_or_panic();
    let runtime_handles = runtime.handles();
    let cache = runtime_handles.cache;
    let refresh_registry = runtime_handles.refresh_registry;

    // Track previous cache key for cleanup
    let mut prev_cache_key = use_signal(|| String::new());

    // Use memo with reactive dependencies to track changes automatically
    let runtime_for_memo = runtime.clone();
    let cache_for_memo = cache.clone();
    let refresh_for_memo = refresh_registry.clone();

    let _execution_memo = use_memo(use_reactive!(|(provider, param)| {
        let runtime = runtime_for_memo.clone();
        let cache = cache_for_memo.clone();
        let refresh_registry = refresh_for_memo.clone();
        let cache_key = provider.id(&param);

        // Clean up previous cache key's tasks if it changed
        let prev_key = prev_cache_key.read().clone();
        if prev_key != cache_key {
            if !prev_key.is_empty() {
                runtime.stop_provider_tasks(&prev_key);
                crate::debug_log!(
                    "🧹 [CLEANUP] Stopped all tasks for previous cache key: {}",
                    prev_key
                );
            }

            // Only update tracked cache key if it actually changed to avoid unnecessary re-renders
            prev_cache_key.set(cache_key.clone());
        }

        runtime.ensure_provider_tasks(&provider, &param, &cache_key);

        // Subscribe to refresh events for this cache key if we have a reactive context
        if let Some(reactive_context) = ReactiveContext::current() {
            refresh_registry.subscribe_to_refresh(&cache_key, reactive_context);
        }

        // Read the current refresh count (this makes the memo reactive to changes)
        let _current_refresh_count = refresh_registry.get_refresh_count(&cache_key);

        // Note: We don't check expiration or SWR here to avoid loops
        // - Cache expiration is handled by the periodic cache expiration task
        // - SWR staleness checking is handled by the periodic stale check task
        // - These periodic tasks run in the background without causing re-render loops

        // Check cache for valid data
        if let Some(cached_result) = cache.get::<Result<P::Output, P::Error>>(&cache_key) {
            // Access tracking is automatically handled by cache.get() updating last_accessed time
            // Removed verbose cache hit logging to reduce spam

            match cached_result {
                Ok(data) => {
                    // Only update state if it's different to avoid unnecessary re-renders
                    if !matches!(*state.read(), State::Success(ref d) if d == &data) {
                        state.set(State::Success(data));
                    }
                }
                Err(error) => {
                    // Only update state if it's different to avoid unnecessary re-renders
                    if !matches!(*state.read(), State::Error(ref e) if e == &error) {
                        state.set(State::Error(error));
                    }
                }
            }
            return;
        }

        // Delegate cache miss orchestration to the runtime so hooks stay lean
        handle_cache_miss(
            &runtime,
            provider.clone(),
            param.clone(),
            cache.clone(),
            refresh_registry.clone(),
            cache_key.clone(),
            state.clone(),
        );
    }));

    state
}

/// Performs SWR staleness checking and triggers background revalidation if needed
/// Unified hook for using any provider - automatically detects parameterized vs non-parameterized providers
///
/// This is the main hook for consuming providers in Dioxus components. It automatically
/// handles both simple providers (no parameters) and parameterized providers, providing
/// a consistent interface for all provider types through the `IntoProviderParam` trait.
///
/// ## Supported Parameter Formats
///
/// - **No parameters**: `use_provider(provider, ())`
/// - **Tuple parameters**: `use_provider(provider, (param,))`
/// - **Direct parameters**: `use_provider(provider, param)`
///
/// ## Features
///
/// - **Automatic Caching**: Results are cached based on provider configuration
/// - **Reactive Updates**: Components automatically re-render when data changes
/// - **Loading States**: Provides loading, success, and error states
/// - **Background Refresh**: Supports interval refresh and stale-while-revalidate
/// - **Auto-Dispose**: Automatically cleans up unused providers
/// - **Unified API**: Single function handles all parameter formats
///
/// ## Usage Examples
///
/// ```rust,no_run
/// use dioxus::prelude::*;
/// use dioxus_provider::prelude::*;
///
/// #[provider]
/// async fn fetch_user() -> Result<String, String> {
///     Ok("User data".to_string())
/// }
///
/// #[provider]
/// async fn fetch_user_by_id(user_id: u32) -> Result<String, String> {
///     Ok(format!("User {}", user_id))
/// }
///
/// #[component]
/// fn MyComponent() -> Element {
///     // All of these work seamlessly:
///     let user = use_provider(fetch_user(), ());           // No parameters
///     let user_by_id = use_provider(fetch_user_by_id(), 123);     // Direct parameter
///     let user_by_id_tuple = use_provider(fetch_user_by_id(), (123,)); // Tuple parameter
///
///     rsx! {
///         div { "Users loaded!" }
///     }
/// }
/// ```
pub fn use_provider<P, Args>(provider: P, args: Args) -> Signal<State<P::Output, P::Error>>
where
    P: UseProvider<Args>,
{
    provider.use_provider(args)
}
