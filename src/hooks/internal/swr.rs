//! Stale-while-revalidate (SWR) functionality

use dioxus::prelude::*;
use std::time::Duration;


use crate::{cache::ProviderCache, refresh::RefreshRegistry, types::ProviderParamBounds};

use super::super::Provider;

/// Check and handle stale-while-revalidate logic
///
/// This function implements the SWR pattern where stale data is served immediately
/// while fresh data is fetched in the background. If data is stale but not expired
/// and no revalidation is in progress, it triggers a background revalidation.
pub fn check_and_handle_swr_core<P, Param>(
    provider: &P,
    param: &Param,
    cache_key: &str,
    cache: &ProviderCache,
    refresh_registry: &RefreshRegistry,
) where
    P: Provider<Param> + Clone,
    Param: ProviderParamBounds,
{
    let stale_time = provider.stale_time();
    let cache_expiration = provider.cache_expiration();

    if let Some(stale_duration) = stale_time {
        if let Ok(cache_lock) = cache.cache.lock() {
            if let Some(entry) = cache_lock.get(cache_key) {
                if entry.is_stale(stale_duration)
                    && !entry.is_expired(cache_expiration.unwrap_or(Duration::from_secs(3600)))
                    && !refresh_registry.is_revalidation_in_progress(cache_key)
                {
                    // Data is stale but not expired and no revalidation in progress - trigger background revalidation
                    if refresh_registry.start_revalidation(cache_key) {
                        crate::debug_log!(
                            "🔄 [SWR] Data is stale for key: {} - triggering background revalidation",
                            cache_key
                        );

                        let cache = cache.clone();
                        let cache_key_clone = cache_key.to_string();
                        let provider = provider.clone();
                        let param = param.clone();
                        let refresh_registry_clone = refresh_registry.clone();

                        spawn(async move {
                            let result = provider.run(param).await;
                            let updated = cache.set(cache_key_clone.clone(), result);
                            refresh_registry_clone.complete_revalidation(&cache_key_clone);
                            if updated {
                                refresh_registry_clone.trigger_refresh(&cache_key_clone);
                                crate::debug_log!(
                                    "✅ [SWR] Background revalidation completed for key: {} (value changed)",
                                    cache_key_clone
                                );
                            } else {
                                crate::debug_log!(
                                    "✅ [SWR] Background revalidation completed for key: {} (value unchanged)",
                                    cache_key_clone
                                );
                            }
                        });
                    }
                }
            }
        }
    }
}
