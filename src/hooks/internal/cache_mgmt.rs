//! Cache management utilities

use std::time::Duration;
#[cfg(feature = "tracing")]
use tracing::debug;

use crate::{
    cache::ProviderCache,
    refresh::{RefreshRegistry, TaskType},
    types::ProviderParamBounds,
};

use super::super::Provider;

/// Sets up intelligent cache management for a provider
///
/// This replaces the old component-unmount auto-dispose with a better system:
/// 1. Access-time tracking for LRU management
/// 2. Periodic cleanup of unused entries based on cache_expiration
/// 3. Cache size limits with LRU eviction
/// 4. Automatic background cleanup tasks
pub fn setup_intelligent_cache_management<P, Param>(
    provider: &P,
    cache_key: &str,
    cache: &ProviderCache,
    refresh_registry: &RefreshRegistry,
) where
    P: Provider<Param> + Clone,
    Param: ProviderParamBounds,
{
    // Set up periodic cleanup task for this provider if cache_expiration is configured
    if let Some(cache_expiration) = provider.cache_expiration() {
        let cleanup_interval = std::cmp::max(
            cache_expiration / 4,    // Clean up 4x more frequently than expiration
            Duration::from_secs(30), // But at least every 30 seconds
        );

        let cache_clone = cache.clone();
        let unused_threshold = cache_expiration * 2; // Remove entries unused for 2x expiration time
        let cleanup_key = format!("{cache_key}_cleanup");

        refresh_registry.start_periodic_task(
            &cleanup_key,
            TaskType::CacheCleanup,
            cleanup_interval,
            move || {
                // Remove entries that haven't been accessed recently
                let removed = cache_clone.cleanup_unused_entries(unused_threshold);
                if removed > 0 {
                    crate::debug_log!(
                        "🧹 [SMART-CLEANUP] Removed {} unused cache entries",
                        removed
                    );
                }

                // Enforce cache size limits (configurable - could be made dynamic)
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
            cache_key, cleanup_interval
        );
    }
}
