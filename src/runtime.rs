//! Shared runtime components that back the provider system.
//!
//! This module lets us manage cache, refresh, and dependency injection handles from one place.

pub mod cache_mgmt;
pub mod request;
pub mod swr;
pub mod tasks;

use crate::{
    cache::ProviderCache,
    hooks::Provider,
    refresh::{RefreshRegistry, TaskType},
    types::ProviderParamBounds,
};
use cache_mgmt::setup_intelligent_cache_management;
use tasks::{
    setup_cache_expiration_task_core, setup_interval_task_core, setup_stale_check_task_core,
};

/// Configuration for the provider runtime.
#[derive(Debug, Clone)]
pub struct ProviderRuntimeConfig {
    enable_dependency_injection: bool,
}

impl ProviderRuntimeConfig {
    /// Create a new config with default settings.
    pub fn new() -> Self {
        Self {
            enable_dependency_injection: false,
        }
    }

    /// Enable dependency injection support for the runtime.
    pub fn with_dependency_injection(mut self) -> Self {
        self.enable_dependency_injection = true;
        self
    }

    pub(crate) fn dependency_injection_enabled(&self) -> bool {
        self.enable_dependency_injection
    }
}

impl Default for ProviderRuntimeConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Central runtime that holds onto core singletons.
#[derive(Clone)]
pub struct ProviderRuntime {
    cache: ProviderCache,
    refresh_registry: RefreshRegistry,
    pending_requests: Arc<Mutex<HashMap<String, u32>>>,
}

/// Lightweight clones of the runtime handles for consumer code.
#[derive(Clone)]
pub struct ProviderRuntimeHandles {
    pub cache: ProviderCache,
    pub refresh_registry: RefreshRegistry,
}

impl ProviderRuntime {
    /// Construct a new runtime instance using the provided configuration.
    pub fn new(config: ProviderRuntimeConfig) -> Self {
        if config.dependency_injection_enabled() {
            crate::injection::ensure_dependency_injection_initialized();
        }

        Self {
            cache: ProviderCache::new(),
            refresh_registry: RefreshRegistry::new(),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Access the cache handle.
    pub fn cache(&self) -> &ProviderCache {
        &self.cache
    }

    /// Access the refresh registry handle.
    pub fn refresh_registry(&self) -> &RefreshRegistry {
        &self.refresh_registry
    }

    /// Get cloned handles for cache and refresh registry.
    pub fn handles(&self) -> ProviderRuntimeHandles {
        ProviderRuntimeHandles {
            cache: self.cache.clone(),
            refresh_registry: self.refresh_registry.clone(),
        }
    }

    /// Stop all scheduled tasks for a cache key.
    pub fn stop_provider_tasks(&self, cache_key: &str) {
        self.refresh_registry.stop_interval_task(cache_key);
        self.refresh_registry
            .stop_periodic_task(cache_key, TaskType::CacheExpiration);
        self.refresh_registry
            .stop_periodic_task(cache_key, TaskType::StaleCheck);

        let cleanup_key = format!("{cache_key}_cleanup");
        self.refresh_registry
            .stop_periodic_task(&cleanup_key, TaskType::CacheCleanup);
    }

    /// Track whether a request for a cache key is already pending.
    pub fn mark_request_pending(&self, cache_key: &str) -> bool {
        if let Ok(mut pending) = self.pending_requests.lock() {
            let count = pending.entry(cache_key.to_string()).or_insert(0);
            *count += 1;
            *count == 1
        } else {
            false
        }
    }

    /// Complete a pending request and return the number of waiters that were affected.
    pub fn mark_request_complete(&self, cache_key: &str) {
        if let Ok(mut pending) = self.pending_requests.lock() {
            if pending.remove(cache_key).is_some() {
                crate::debug_log!(
                    "✅ [REQUEST-DEDUP] Request completed for key: {}",
                    cache_key
                );
            }
        }
    }

    /// Number of components waiting on a given cache key.
    pub fn pending_request_count(&self, cache_key: &str) -> u32 {
        if let Ok(pending) = self.pending_requests.lock() {
            *pending.get(cache_key).unwrap_or(&0)
        } else {
            0
        }
    }

    /// Ensure scheduled tasks are registered for a provider key (native targets).
    #[cfg(not(target_family = "wasm"))]
    pub fn ensure_provider_tasks<P, Param>(&self, provider: &P, param: &Param, cache_key: &str)
    where
        P: Provider<Param> + Clone + Send,
        Param: ProviderParamBounds,
    {
        setup_intelligent_cache_management(
            provider,
            cache_key,
            &self.cache,
            &self.refresh_registry,
        );
        setup_cache_expiration_task_core(
            provider,
            param,
            cache_key,
            &self.cache,
            &self.refresh_registry,
        );
        setup_interval_task_core(
            provider,
            param,
            cache_key,
            &self.cache,
            &self.refresh_registry,
        );
        setup_stale_check_task_core(
            provider,
            param,
            cache_key,
            &self.cache,
            &self.refresh_registry,
        );
    }

    /// Ensure scheduled tasks are registered for a provider key (WASM targets).
    #[cfg(target_family = "wasm")]
    pub fn ensure_provider_tasks<P, Param>(&self, provider: &P, param: &Param, cache_key: &str)
    where
        P: Provider<Param> + Clone,
        Param: ProviderParamBounds,
    {
        setup_intelligent_cache_management(
            provider,
            cache_key,
            &self.cache,
            &self.refresh_registry,
        );
        setup_cache_expiration_task_core(
            provider,
            param,
            cache_key,
            &self.cache,
            &self.refresh_registry,
        );
        setup_interval_task_core(
            provider,
            param,
            cache_key,
            &self.cache,
            &self.refresh_registry,
        );
        setup_stale_check_task_core(
            provider,
            param,
            cache_key,
            &self.cache,
            &self.refresh_registry,
        );
    }
}
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
