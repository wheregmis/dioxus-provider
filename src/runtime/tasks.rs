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
    interval: Duration,
) where
    P: Provider<Param> + Clone + Send,
    Param: ProviderParamBounds,
{
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

#[cfg(target_family = "wasm")]
pub fn setup_interval_task_core<P, Param>(
    provider: &P,
    param: &Param,
    cache_key: &str,
    cache: &ProviderCache,
    refresh_registry: &RefreshRegistry,
    interval: Duration,
) where
    P: Provider<Param> + Clone,
    Param: ProviderParamBounds,
{
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

#[cfg(not(target_family = "wasm"))]
pub fn setup_cache_expiration_task_core<P, Param>(
    _provider: &P,
    _param: &Param,
    cache_key: &str,
    cache: &ProviderCache,
    refresh_registry: &RefreshRegistry,
    expiration: Duration,
) where
    P: Provider<Param> + Clone + Send,
    Param: ProviderParamBounds,
{
    let cache_clone = cache.clone();
    let cache_key_clone = cache_key.to_string();
    let refresh_registry_clone = refresh_registry.clone();

    let check_interval = std::cmp::max(expiration / 4, MIN_TASK_INTERVAL);

    refresh_registry.start_periodic_task(
        cache_key,
        TaskType::CacheExpiration,
        check_interval,
        move || {
            check_and_handle_cache_expiration(
                Some(expiration),
                &cache_key_clone,
                &cache_clone,
                &refresh_registry_clone,
            );
        },
    );
}

#[cfg(target_family = "wasm")]
pub fn setup_cache_expiration_task_core<P, Param>(
    _provider: &P,
    _param: &Param,
    cache_key: &str,
    cache: &ProviderCache,
    refresh_registry: &RefreshRegistry,
    expiration: Duration,
) where
    P: Provider<Param> + Clone,
    Param: ProviderParamBounds,
{
    let cache_clone = cache.clone();
    let cache_key_clone = cache_key.to_string();
    let refresh_registry_clone = refresh_registry.clone();

    let check_interval = std::cmp::max(expiration / 4, MIN_TASK_INTERVAL);

    refresh_registry.start_periodic_task(
        cache_key,
        TaskType::CacheExpiration,
        check_interval,
        move || {
            check_and_handle_cache_expiration(
                Some(expiration),
                &cache_key_clone,
                &cache_clone,
                &refresh_registry_clone,
            );
        },
    );
}

#[cfg(not(target_family = "wasm"))]
pub fn setup_stale_check_task_core<P, Param>(
    provider: &P,
    param: &Param,
    cache_key: &str,
    cache: &ProviderCache,
    refresh_registry: &RefreshRegistry,
    stale_time: Duration,
) where
    P: Provider<Param> + Clone + Send,
    Param: ProviderParamBounds,
{
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

#[cfg(target_family = "wasm")]
pub fn setup_stale_check_task_core<P, Param>(
    provider: &P,
    param: &Param,
    cache_key: &str,
    cache: &ProviderCache,
    refresh_registry: &RefreshRegistry,
    stale_time: Duration,
) where
    P: Provider<Param> + Clone,
    Param: ProviderParamBounds,
{
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

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::*;
    use crate::{cache::ProviderCache, refresh::RefreshRegistry};
    use std::future::Future;
    use tokio::time::sleep;

    fn block_on<F: Future<Output = ()>>(future: F) {
        tokio::runtime::Runtime::new()
            .expect("tokio runtime")
            .block_on(future);
    }

    #[test]
    fn removes_expired_entries_and_triggers_refresh() {
        block_on(async {
            let cache = ProviderCache::new();
            let refresh_registry = RefreshRegistry::new();
            let cache_key = "ttl-expired";
            cache.set(cache_key.to_string(), Ok::<u32, ()>(1));

            sleep(Duration::from_millis(30)).await;

            check_and_handle_cache_expiration(
                Some(Duration::from_millis(10)),
                cache_key,
                &cache,
                &refresh_registry,
            );

            assert!(
                cache
                    .get::<Result<u32, ()>>(cache_key)
                    .is_none(),
                "expired entries should be removed from cache"
            );
            assert_eq!(
                refresh_registry.get_refresh_count(cache_key),
                1,
                "removing expired data should trigger a refresh"
            );
        });
    }

    #[test]
    fn keeps_fresh_entries_without_refresh() {
        block_on(async {
            let cache = ProviderCache::new();
            let refresh_registry = RefreshRegistry::new();
            let cache_key = "ttl-fresh";
            cache.set(cache_key.to_string(), Ok::<u32, ()>(5));

            sleep(Duration::from_millis(5)).await;

            check_and_handle_cache_expiration(
                Some(Duration::from_millis(50)),
                cache_key,
                &cache,
                &refresh_registry,
            );

            let cached = cache
                .get::<Result<u32, ()>>(cache_key)
                .expect("entry should still exist");
            assert_eq!(cached, Ok(5));
            assert_eq!(
                refresh_registry.get_refresh_count(cache_key),
                0,
                "no refresh should fire when TTL not reached"
            );
        });
    }
}
