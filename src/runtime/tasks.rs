//! Task management for provider background operations.

use dioxus::prelude::*;
use std::time::Duration;

use crate::{
    cache::ProviderCache,
    hooks::Provider,
    refresh::{RefreshRegistry, TaskType},
    runtime::swr::check_and_handle_swr_core,
    types::ProviderParamBounds,
};

/// Minimum interval for periodic tasks to prevent busy spinning.
const MIN_TASK_INTERVAL: Duration = Duration::from_millis(1);

#[cfg(not(target_family = "wasm"))]
pub fn setup_interval_task_core<P, Param>(
    provider: &P,
    param: &Param,
    cache_key: &str,
    cache: &ProviderCache,
    refresh_registry: &RefreshRegistry,
) where
    P: Provider<Param> + Clone + Send,
    Param: ProviderParamBounds,
{
    if let Some(interval) = provider.interval() {
        let cache_clone = cache.clone();
        let provider_clone = provider.clone();
        let param_clone = param.clone();
        let cache_key_clone = cache_key.to_string();
        let refresh_registry_clone = refresh_registry.clone();

        refresh_registry.start_interval_task(cache_key, interval, move || {
            let cache_for_task = cache_clone.clone();
            let provider_for_task = provider_clone.clone();
            let param_for_task = param_clone.clone();
            let cache_key_for_task = cache_key_clone.clone();
            let refresh_registry_for_task = refresh_registry_clone.clone();

            spawn(async move {
                let result = provider_for_task.run(param_for_task).await;
                let updated = cache_for_task.set(cache_key_for_task.clone(), result);
                if updated {
                    refresh_registry_for_task.trigger_refresh(&cache_key_for_task);
                }
            });
        });
    }
}

#[cfg(target_family = "wasm")]
pub fn setup_interval_task_core<P, Param>(
    provider: &P,
    param: &Param,
    cache_key: &str,
    cache: &ProviderCache,
    refresh_registry: &RefreshRegistry,
) where
    P: Provider<Param> + Clone,
    Param: ProviderParamBounds,
{
    if let Some(interval) = provider.interval() {
        let cache_clone = cache.clone();
        let provider_clone = provider.clone();
        let param_clone = param.clone();
        let cache_key_clone = cache_key.to_string();
        let refresh_registry_clone = refresh_registry.clone();

        refresh_registry.start_interval_task(cache_key, interval, move || {
            let cache_for_task = cache_clone.clone();
            let provider_for_task = provider_clone.clone();
            let param_for_task = param_clone.clone();
            let cache_key_for_task = cache_key_clone.clone();
            let refresh_registry_for_task = refresh_registry_clone.clone();

            spawn(async move {
                let result = provider_for_task.run(param_for_task).await;
                let updated = cache_for_task.set(cache_key_for_task.clone(), result);
                if updated {
                    refresh_registry_for_task.trigger_refresh(&cache_key_for_task);
                }
            });
        });
    }
}

#[cfg(not(target_family = "wasm"))]
pub fn setup_cache_expiration_task_core<P, Param>(
    provider: &P,
    _param: &Param,
    cache_key: &str,
    cache: &ProviderCache,
    refresh_registry: &RefreshRegistry,
) where
    P: Provider<Param> + Clone + Send,
    Param: ProviderParamBounds,
{
    if let Some(expiration) = provider.cache_expiration() {
        let cache_clone = cache.clone();
        let cache_key_clone = cache_key.to_string();
        let refresh_registry_clone = refresh_registry.clone();

        let check_interval = std::cmp::max(expiration / 4, MIN_TASK_INTERVAL);

        refresh_registry.start_periodic_task(
            cache_key,
            TaskType::CacheExpiration,
            check_interval,
            move || {
                if let Ok(mut cache_lock) = cache_clone.cache.lock() {
                    if let Some(entry) = cache_lock.get(&cache_key_clone) {
                        if entry.is_expired(expiration) {
                            crate::debug_log!(
                                "🗑️ [AUTO-EXPIRATION] Cache expired for key: {} - triggering reactive refresh",
                                cache_key_clone
                            );
                            cache_lock.remove(&cache_key_clone);
                            drop(cache_lock);
                            refresh_registry_clone.trigger_refresh(&cache_key_clone);
                        }
                    }
                }
            },
        );
    }
}

#[cfg(target_family = "wasm")]
pub fn setup_cache_expiration_task_core<P, Param>(
    provider: &P,
    _param: &Param,
    cache_key: &str,
    cache: &ProviderCache,
    refresh_registry: &RefreshRegistry,
) where
    P: Provider<Param> + Clone,
    Param: ProviderParamBounds,
{
    if let Some(expiration) = provider.cache_expiration() {
        let cache_clone = cache.clone();
        let cache_key_clone = cache_key.to_string();
        let refresh_registry_clone = refresh_registry.clone();

        let check_interval = std::cmp::max(expiration / 4, MIN_TASK_INTERVAL);

        refresh_registry.start_periodic_task(
            cache_key,
            TaskType::CacheExpiration,
            check_interval,
            move || {
                if let Ok(mut cache_lock) = cache_clone.cache.lock() {
                    if let Some(entry) = cache_lock.get(&cache_key_clone) {
                        if entry.is_expired(expiration) {
                            crate::debug_log!(
                                "🗑️ [AUTO-EXPIRATION] Cache expired for key: {} - triggering reactive refresh",
                                cache_key_clone
                            );
                            cache_lock.remove(&cache_key_clone);
                            drop(cache_lock);
                            refresh_registry_clone.trigger_refresh(&cache_key_clone);
                        }
                    }
                }
            },
        );
    }
}

#[cfg(not(target_family = "wasm"))]
pub fn setup_stale_check_task_core<P, Param>(
    provider: &P,
    param: &Param,
    cache_key: &str,
    cache: &ProviderCache,
    refresh_registry: &RefreshRegistry,
) where
    P: Provider<Param> + Clone + Send,
    Param: ProviderParamBounds,
{
    if let Some(stale_time) = provider.stale_time() {
        let cache_clone = cache.clone();
        let provider_clone = provider.clone();
        let param_clone = param.clone();
        let cache_key_clone = cache_key.to_string();
        let refresh_registry_clone = refresh_registry.clone();

        refresh_registry.start_stale_check_task(cache_key, stale_time, move || {
            check_and_handle_swr_core(
                &provider_clone,
                &param_clone,
                &cache_key_clone,
                &cache_clone,
                &refresh_registry_clone,
            );
        });
    }
}

#[cfg(target_family = "wasm")]
pub fn setup_stale_check_task_core<P, Param>(
    provider: &P,
    param: &Param,
    cache_key: &str,
    cache: &ProviderCache,
    refresh_registry: &RefreshRegistry,
) where
    P: Provider<Param> + Clone,
    Param: ProviderParamBounds,
{
    if let Some(stale_time) = provider.stale_time() {
        let cache_clone = cache.clone();
        let provider_clone = provider.clone();
        let param_clone = param.clone();
        let cache_key_clone = cache_key.to_string();
        let refresh_registry_clone = refresh_registry.clone();

        refresh_registry.start_stale_check_task(cache_key, stale_time, move || {
            check_and_handle_swr_core(
                &provider_clone,
                &param_clone,
                &cache_key_clone,
                &cache_clone,
                &refresh_registry_clone,
            );
        });
    }
}

#[allow(dead_code)]
pub fn check_and_handle_cache_expiration(
    cache_expiration: Option<Duration>,
    cache_key: &str,
    cache: &ProviderCache,
    refresh_registry: &RefreshRegistry,
) {
    if let Some(expiration) = cache_expiration {
        let should_trigger_refresh = if let Ok(mut cache_lock) = cache.cache.lock() {
            if let Some(entry) = cache_lock.get(cache_key) {
                if entry.is_expired(expiration) {
                    crate::debug_log!(
                        "🗑️ [CACHE EXPIRATION] Removing expired cache entry for key: {}",
                        cache_key
                    );
                    cache_lock.remove(cache_key);
                    true
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

        if should_trigger_refresh {
            refresh_registry.trigger_refresh(cache_key);
        }
    }
}
