//! Cache management utilities owned by the runtime.

use std::time::Duration;

use crate::{
    cache::ProviderCache,
    refresh::{RefreshRegistry, TaskType},
};

/// Sets up intelligent cache management for a provider.
///
/// This replaces the old component-unmount auto-dispose with a better system:
/// 1. Access-time tracking for LRU management
/// 2. Periodic cleanup of unused entries based on cache_expiration
/// 3. Cache size limits with LRU eviction
/// 4. Automatic background cleanup tasks
pub fn setup_intelligent_cache_management(
    cache_expiration: Duration,
    cache_key: &str,
    cache: &ProviderCache,
    refresh_registry: &RefreshRegistry,
) {
    let cleanup_interval = std::cmp::max(cache_expiration / 4, Duration::from_secs(30));

    let cache_clone = cache.clone();
    let unused_threshold = cache_expiration * 2;
    let cleanup_key = format!("{cache_key}_cleanup");

    refresh_registry.start_periodic_task(
        &cleanup_key,
        TaskType::CacheCleanup,
        cleanup_interval,
        move || {
            let removed = cache_clone.cleanup_unused_entries(unused_threshold);
            if removed > 0 {
                crate::debug_log!(
                    "🧹 [SMART-CLEANUP] Removed {} unused cache entries",
                    removed
                );
            }

            const MAX_CACHE_SIZE: usize = 1000;
            let evicted = cache_clone.evict_lru_entries(MAX_CACHE_SIZE);
            if evicted > 0 {
                crate::debug_log!(
                    "🗑️ [LRU-EVICT] Evicted {} entries due to cache size limit",
                    evicted
                );
            }
        },
    );

    #[cfg(feature = "tracing")]
    crate::debug_log!(
        "📊 [SMART-CACHE] Intelligent cache management enabled for: {} (cleanup every {:?})",
        cache_key,
        cleanup_interval
    );
}
