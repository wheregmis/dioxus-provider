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
use tracing::debug;

use crate::{
    cache::ProviderCache,
    global::{get_global_cache, get_global_refresh_registry},
    refresh::RefreshRegistry,
};

use crate::param_utils::IntoProviderParam;
use crate::types::{ProviderErrorBounds, ProviderOutputBounds, ProviderParamBounds};

// Import helper functions from internal modules
use super::internal::cache_mgmt::setup_intelligent_cache_management;
use super::internal::swr::check_and_handle_swr_core;
use super::internal::tasks::{
    check_and_handle_cache_expiration, setup_cache_expiration_task_core, setup_interval_task_core,
    setup_stale_check_task_core,
};

pub use crate::provider_state::ProviderState;

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
    /// hashes the provider's type and parameters to generate a unique ID.
    fn id(&self, param: &Param) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        std::any::TypeId::of::<Self>().hash(&mut hasher);
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
impl<T: Clone + 'static, E: Clone + 'static> SuspenseSignalExt<T, E>
    for Signal<ProviderState<T, E>>
{
    fn suspend(&self) -> Result<Result<T, E>, RenderError> {
        match &*self.read() {
            ProviderState::Loading { task } => {
                Err(RenderError::Suspended(SuspendedFuture::new(*task)))
            }
            ProviderState::Success(data) => Ok(Ok(data.clone())),
            ProviderState::Error(error) => Ok(Err(error.clone())),
        }
    }
}

/// Get the provider cache - requires global providers to be initialized
fn get_provider_cache() -> ProviderCache {
    get_global_cache()
        .unwrap_or_else(|_| {
            panic!("Global providers not initialized. Call dioxus_provider::init() before using providers.")
        })
        .clone()
}

/// Get the refresh registry - requires global providers to be initialized
fn get_refresh_registry() -> RefreshRegistry {
    get_global_refresh_registry()
        .unwrap_or_else(|_| {
            panic!("Global providers not initialized. Call dioxus_provider::init() before using providers.")
        })
        .clone()
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
    let cache = get_provider_cache();
    let refresh_registry = get_refresh_registry();
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
    let cache = get_provider_cache();
    let refresh_registry = get_refresh_registry();

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
    fn use_provider(self, args: Args) -> Signal<ProviderState<Self::Output, Self::Error>>;
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

    fn use_provider(self, args: Args) -> Signal<ProviderState<Self::Output, Self::Error>> {
        let param = args.into_param();
        use_provider_core(self, param)
    }
}

/// Core provider implementation that handles all the common logic
fn use_provider_core<P, Param>(
    provider: P,
    param: Param,
) -> Signal<ProviderState<P::Output, P::Error>>
where
    P: Provider<Param> + Send + Clone,
    Param: ProviderParamBounds,
{
    let mut state = use_signal(|| ProviderState::Loading {
        task: spawn(async {}),
    });
    let cache = get_provider_cache();
    let refresh_registry = get_refresh_registry();

    let cache_key = provider.id(&param);
    let cache_expiration = provider.cache_expiration();

    // Setup intelligent cache management (replaces old auto-dispose system)
    setup_intelligent_cache_management(&provider, &cache_key, &cache, &refresh_registry);

    // Check cache expiration before the memo - this happens on every render
    check_and_handle_cache_expiration(cache_expiration, &cache_key, &cache, &refresh_registry);

    // SWR staleness checking - runs on every render to check for stale data
    check_and_handle_swr_core(&provider, &param, &cache_key, &cache, &refresh_registry);

    // Use memo with reactive dependencies to track changes automatically
    let _execution_memo = use_memo(use_reactive!(|(provider, param)| {
        let cache_key = provider.id(&param);

        debug!(
            "🔄 [USE_PROVIDER] Memo executing for key: {} with param: {:?}",
            cache_key, param
        );

        // Subscribe to refresh events for this cache key if we have a reactive context
        if let Some(reactive_context) = ReactiveContext::current() {
            refresh_registry.subscribe_to_refresh(&cache_key, reactive_context);
        }

        // Read the current refresh count (this makes the memo reactive to changes)
        let _current_refresh_count = refresh_registry.get_refresh_count(&cache_key);

        // Set up cache expiration monitoring task
        setup_cache_expiration_task_core(&provider, &param, &cache_key, &cache, &refresh_registry);

        // Set up interval task if provider has interval configured
        setup_interval_task_core(&provider, &param, &cache_key, &cache, &refresh_registry);

        // Set up stale check task if provider has stale time configured
        setup_stale_check_task_core(&provider, &param, &cache_key, &cache, &refresh_registry);

        // Check cache for valid data
        if let Some(cached_result) = cache.get::<Result<P::Output, P::Error>>(&cache_key) {
            // Access tracking is automatically handled by cache.get() updating last_accessed time
            debug!("📊 [CACHE-HIT] Serving cached data for: {}", cache_key);

            match cached_result {
                Ok(data) => {
                    let _ = spawn(async move {
                        state.set(ProviderState::Success(data));
                    });
                }
                Err(error) => {
                    let _ = spawn(async move {
                        state.set(ProviderState::Error(error));
                    });
                }
            }
            return;
        }

        // Cache miss - set loading and spawn async task
        let cache_clone = cache.clone();
        let cache_key_clone = cache_key.clone();
        let provider = provider.clone();
        let param = param.clone();
        let mut state_for_async = state;

        // Spawn the real async task and store the handle in Loading
        let task = spawn(async move {
            let result = provider.run(param).await;
            let updated = cache_clone.set(cache_key_clone.clone(), result.clone());
            debug!(
                "📊 [CACHE-STORE] Attempted to store new data for: {} (updated: {})",
                cache_key_clone, updated
            );
            if updated {
                // Only update state and trigger rerender if value changed
                match result {
                    Ok(data) => state_for_async.set(ProviderState::Success(data)),
                    Err(error) => state_for_async.set(ProviderState::Error(error)),
                }
            }
        });
        state.set(ProviderState::Loading { task });
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
pub fn use_provider<P, Args>(provider: P, args: Args) -> Signal<ProviderState<P::Output, P::Error>>
where
    P: UseProvider<Args>,
{
    provider.use_provider(args)
}
