//! # Global Provider Management
//!
//! This module provides global singletons for cache, disposal, and refresh management
//! that operate at application scale rather than component lifecycle scale.

use std::sync::OnceLock;

use crate::{cache::ProviderCache, refresh::RefreshRegistry};

/// Error type for global provider operations
#[derive(Debug, thiserror::Error)]
pub enum GlobalProviderError {
    #[error("Global providers not initialized. Call init_global_providers() first.")]
    NotInitialized,
    #[error("Failed to initialize global providers: {0}")]
    InitializationFailed(String),
}

/// Global singleton instance of the provider cache
static GLOBAL_CACHE: OnceLock<ProviderCache> = OnceLock::new();

/// Global singleton instance of the refresh registry
static GLOBAL_REFRESH_REGISTRY: OnceLock<RefreshRegistry> = OnceLock::new();

/// Configuration for initializing the global provider system
#[derive(Default, Debug, Clone)]
pub struct ProviderConfig {
    enable_dependency_injection: bool,
}

impl ProviderConfig {
    /// Create a new provider configuration with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable dependency injection support
    pub fn with_dependency_injection(mut self) -> Self {
        self.enable_dependency_injection = true;
        self
    }

    /// Initialize the global provider system with this configuration
    pub fn init(self) -> Result<(), GlobalProviderError> {
        // Initialize cache first
        GLOBAL_CACHE.get_or_init(ProviderCache::new);

        // Initialize refresh registry
        let _refresh_registry = GLOBAL_REFRESH_REGISTRY.get_or_init(RefreshRegistry::new);

        // Initialize dependency injection if enabled
        if self.enable_dependency_injection {
            crate::injection::init_dependency_injection();
        }

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
    GLOBAL_CACHE
        .get()
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
    GLOBAL_REFRESH_REGISTRY
        .get()
        .ok_or(GlobalProviderError::NotInitialized)
}

/// Check if global providers have been initialized
pub fn is_initialized() -> bool {
    GLOBAL_CACHE.get().is_some() && GLOBAL_REFRESH_REGISTRY.get().is_some()
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
        // If already initialized, just test that we can get the instances
        if is_initialized() {
            let _cache = get_global_cache().unwrap();
            let _refresh = get_global_refresh_registry().unwrap();
            return;
        }

        // Test initialization from scratch
        assert!(!is_initialized());

        init_global_providers().unwrap();

        assert!(is_initialized());

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
