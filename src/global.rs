//! # Global Provider Management
//!
//! This module provides global singletons for cache, disposal, and refresh management
//! that operate at application scale rather than component lifecycle scale.

use std::sync::OnceLock;

use crate::{
    cache::ProviderCache,
    refresh::RefreshRegistry,
    runtime::{ProviderRuntime, ProviderRuntimeConfig, ProviderRuntimeHandles},
};

/// Error type for global provider operations
#[derive(Debug, thiserror::Error)]
pub enum GlobalProviderError {
    #[error("Global providers not initialized. Call init_global_providers() first.")]
    NotInitialized,
    #[error("Failed to initialize global providers: {0}")]
    InitializationFailed(String),
}

/// Global singleton instance of the provider runtime
static GLOBAL_RUNTIME: OnceLock<ProviderRuntime> = OnceLock::new();

/// Configuration for initializing the global provider system
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    runtime_config: ProviderRuntimeConfig,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            runtime_config: ProviderRuntimeConfig::new(),
        }
    }
}

impl ProviderConfig {
    /// Create a new provider configuration with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable dependency injection support
    pub fn with_dependency_injection(mut self) -> Self {
        self.runtime_config = self.runtime_config.clone().with_dependency_injection();
        self
    }

    /// Initialize the global provider system with this configuration
    pub fn init(self) -> Result<(), GlobalProviderError> {
        let runtime_config = self.runtime_config.clone();
        GLOBAL_RUNTIME.get_or_init(|| ProviderRuntime::new(runtime_config));

        Ok(())
    }
}

/// Initialize the global provider system with all features enabled
///
/// This is the recommended way to initialize dioxus-provider. It sets up:
/// - Global cache for provider results
/// - Refresh registry for reactive updates
/// - Dependency injection system
///
/// ## Example
///
/// ```rust,no_run
/// use dioxus::prelude::*;
/// use dioxus_provider::global::init;
///
/// fn main() {
///     // Initialize global provider system
///     init();
///     
///     // Launch your app
///     dioxus::launch(app);
/// }
///
/// #[component]
/// fn app() -> Element {
///     rsx! {
///         div { "Hello World!" }
///     }
/// }
/// ```
pub fn init() -> Result<(), GlobalProviderError> {
    ProviderConfig::new().with_dependency_injection().init()
}

/// Initialize the global provider management system (without dependency injection)
///
/// This should be called once at the start of your application,
/// typically in your main function or app initialization.
///
/// ## Example
///
/// ```rust,no_run
/// use dioxus::prelude::*;
/// use dioxus_provider::global::init_global_providers;
///
/// fn main() {
///     // Initialize global provider system
///     init_global_providers();
///     
///     // Launch your app
///     dioxus::launch(app);
/// }
///
/// #[component]
/// fn app() -> Element {
///     rsx! {
///         div { "Hello World!" }
///     }
/// }
/// ```
#[deprecated(
    since = "0.1.0",
    note = "Use init() or ProviderConfig::new().init() instead"
)]
pub fn init_global_providers() -> Result<(), GlobalProviderError> {
    ProviderConfig::new().init()
}

/// Get the global provider cache instance
///
/// Returns the global cache that persists across the entire application lifecycle.
/// This cache is shared by all providers regardless of component boundaries.
///
/// ## Errors
///
/// Returns `GlobalProviderError::NotInitialized` if `init_global_providers()` has not been called yet.
pub fn get_global_cache() -> Result<&'static ProviderCache, GlobalProviderError> {
    GLOBAL_RUNTIME
        .get()
        .map(|runtime| runtime.cache())
        .ok_or(GlobalProviderError::NotInitialized)
}

/// Get the global refresh registry instance
///
/// Returns the global refresh registry that manages reactive updates and intervals
/// across the entire application.
///
/// ## Errors
///
/// Returns `GlobalProviderError::NotInitialized` if `init_global_providers()` has not been called yet.
pub fn get_global_refresh_registry() -> Result<&'static RefreshRegistry, GlobalProviderError> {
    GLOBAL_RUNTIME
        .get()
        .map(|runtime| runtime.refresh_registry())
        .ok_or(GlobalProviderError::NotInitialized)
}

/// Access the global runtime handle.
pub fn get_global_runtime() -> Result<&'static ProviderRuntime, GlobalProviderError> {
    GLOBAL_RUNTIME
        .get()
        .ok_or(GlobalProviderError::NotInitialized)
}

/// Clone handles to the global runtime for use in hooks and mutations.
pub fn get_global_runtime_handles() -> Result<ProviderRuntimeHandles, GlobalProviderError> {
    get_global_runtime().map(|runtime| runtime.handles())
}

/// Check if global providers have been initialized
pub fn is_initialized() -> bool {
    GLOBAL_RUNTIME.get().is_some()
}

/// Ensure that global providers have been initialized
///
/// This helper function returns an error if the global providers have not been initialized yet.
/// It's useful for providing better error messages in hooks and other functions that depend
/// on the global provider system.
///
/// ## Example
///
/// ```rust,no_run
/// use dioxus_provider::global::ensure_initialized;
/// use dioxus_provider::errors::ProviderError;
///
/// fn my_hook() -> Result<(), ProviderError> {
///     ensure_initialized()?;
///     // ... rest of hook logic
///     Ok(())
/// }
/// ```
pub fn ensure_initialized() -> Result<(), crate::errors::ProviderError> {
    if !is_initialized() {
        return Err(crate::errors::ProviderError::Configuration(
            "Global providers not initialized. Call init() at application startup.".to_string(),
        ));
    }
    Ok(())
}

/// Reset global providers (mainly for testing)
///
/// This is primarily intended for testing scenarios where you need
/// to reset the global state between tests.
///
/// ## Warning
///
/// This function is not thread-safe and should only be used in single-threaded
/// test environments. Do not use this in production code.
#[cfg(test)]
pub fn reset_global_providers() {
    // Note: OnceLock doesn't have a public reset method, so this is mainly
    // for documentation. In real tests, you'd typically use a different
    // approach or restart the test process.
    panic!("Global provider reset is not currently supported. Restart the application.");
}

// Note: Deprecated functions get_global_cache_panic() and get_global_refresh_registry_panic()
// have been removed in version 0.1.0. Use get_global_cache() and get_global_refresh_registry()
// with proper error handling instead.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_provider_initialization() {
        init().unwrap();

        // Test that we can get all instances
        let _cache = get_global_cache().unwrap();
        let _refresh = get_global_refresh_registry().unwrap();
    }

    #[test]
    fn test_error_when_not_initialized() {
        // Check if already initialized - skip test if so
        if is_initialized() {
            return;
        }

        // This should return an error since we haven't called init_global_providers()
        assert!(get_global_cache().is_err());
        assert!(get_global_refresh_registry().is_err());
    }

    #[test]
    fn test_get_functions_with_error_handling() {
        // Test that get functions work with proper error handling
        init().unwrap();

        let _cache = get_global_cache().unwrap();
        let _refresh = get_global_refresh_registry().unwrap();
    }
}
