//! # Cache Management for dioxus-provider
//!
//! This module implements a global, type-erased cache for provider results, supporting:
//! - **Expiration**: Entries are removed after a configurable TTL.
//! - **Staleness (SWR)**: Entries can be marked stale and revalidated in the background.
//! - **LRU Eviction**: Least-recently-used entries are evicted to maintain a size limit.
//! - **Access/Usage Stats**: Provides statistics for cache introspection and tuning.
//!
//! ## Example
//! ```rust,no_run
//! use dioxus_provider::cache::ProviderCache;
//! let cache = ProviderCache::new();
//! cache.set("my_key".to_string(), 42);
//! let value: Option<i32> = cache.get("my_key");
//! ```
//! Cache management and async state types for dioxus-provider

use std::{
    any::Any,
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU32, Ordering},
    },
    time::Duration,
};

use crate::platform::{DEFAULT_MAX_CACHE_SIZE, DEFAULT_UNUSED_THRESHOLD};

// Platform-specific time imports
#[cfg(not(target_family = "wasm"))]
use std::time::Instant;
#[cfg(target_family = "wasm")]
use web_time::Instant;

/// Options for cache retrieval operations
#[derive(Debug, Clone, Default)]
pub struct CacheGetOptions {
    /// Optional expiration duration - entries older than this will be removed
    pub expiration: Option<Duration>,
    /// Optional stale time - used to check if data is stale
    pub stale_time: Option<Duration>,
    /// Whether to return staleness information
    pub check_staleness: bool,
}

impl CacheGetOptions {
    /// Create new cache get options with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the expiration duration
    pub fn with_expiration(mut self, expiration: Duration) -> Self {
        self.expiration = Some(expiration);
        self
    }

    /// Set the stale time
    pub fn with_stale_time(mut self, stale_time: Duration) -> Self {
        self.stale_time = Some(stale_time);
        self.check_staleness = true;
        self
    }

    /// Enable staleness checking
    pub fn check_staleness(mut self) -> Self {
        self.check_staleness = true;
        self
    }
}

/// Result type for cache get operations with staleness information
#[derive(Debug, Clone)]
pub struct CacheGetResult<T> {
    /// The cached data
    pub data: T,
    /// Whether the data is considered stale
    pub is_stale: bool,
}

/// A type-erased cache entry for storing provider results with timestamp and access tracking
#[derive(Clone)]
pub struct CacheEntry {
    data: Arc<dyn Any + Send + Sync>,
    cached_at: Arc<Mutex<Instant>>,
    last_accessed: Arc<Mutex<Instant>>,
    access_count: Arc<AtomicU32>,
}

impl CacheEntry {
    /// Creates a new cache entry with the given data.
    ///
    /// # Arguments
    ///
    /// * `data` - The data to cache.
    ///
    /// # Returns
    ///
    /// A new `CacheEntry` instance.
    pub fn new<T: Clone + Send + Sync + 'static>(data: T) -> Self {
        let now = Instant::now();
        Self {
            data: Arc::new(data),
            cached_at: Arc::new(Mutex::new(now)),
            last_accessed: Arc::new(Mutex::new(now)),
            access_count: Arc::new(AtomicU32::new(0)),
        }
    }

    /// Retrieves the cached data of type `T`.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `CacheEntry`.
    ///
    /// # Returns
    ///
    /// An `Option<T>` containing the cached data if available, or `None` if the entry is expired or not found.
    ///
    /// # Side Effects
    ///
    /// Updates the `last_accessed` timestamp and increments the `access_count`.
    pub fn get<T: Clone + Send + Sync + 'static>(&self) -> Option<T> {
        // Update last accessed time and access count
        if let Ok(mut last_accessed) = self.last_accessed.lock() {
            *last_accessed = Instant::now();
        }
        self.access_count.fetch_add(1, Ordering::SeqCst);
        self.data.downcast_ref::<T>().cloned()
    }

    /// Refreshes the cached_at timestamp to the current time.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `CacheEntry`.
    ///
    /// # Side Effects
    ///
    /// Updates the `cached_at` timestamp to the current time.
    pub fn refresh_timestamp(&self) {
        if let Ok(mut cached_at) = self.cached_at.lock() {
            *cached_at = Instant::now();
        }
    }

    /// Checks if the cache entry has expired based on the given expiration duration.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `CacheEntry`.
    /// * `expiration` - The duration after which the entry is considered expired.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the entry has expired.
    pub fn is_expired(&self, expiration: Duration) -> bool {
        if let Ok(cached_at) = self.cached_at.lock() {
            cached_at.elapsed() > expiration
        } else {
            false
        }
    }

    /// Checks if the cache entry is stale based on the given stale time.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `CacheEntry`.
    /// * `stale_time` - The duration after which the entry is considered stale.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the entry is stale.
    pub fn is_stale(&self, stale_time: Duration) -> bool {
        if let Ok(cached_at) = self.cached_at.lock() {
            cached_at.elapsed() > stale_time
        } else {
            false
        }
    }

    /// Gets the current access count for the cache entry.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `CacheEntry`.
    ///
    /// # Returns
    ///
    /// The current access count as a `u32`.
    pub fn access_count(&self) -> u32 {
        self.access_count.load(Ordering::SeqCst)
    }

    /// Checks if the cache entry hasn't been accessed for the given duration.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `CacheEntry`.
    /// * `duration` - The duration after which the entry is considered unused.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the entry is unused.
    pub fn is_unused_for(&self, duration: Duration) -> bool {
        if let Ok(last_accessed) = self.last_accessed.lock() {
            last_accessed.elapsed() > duration
        } else {
            false
        }
    }

    /// Gets the time since this entry was last accessed.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `CacheEntry`.
    ///
    /// # Returns
    ///
    /// A `Duration` representing the time since last access.
    pub fn time_since_last_access(&self) -> Duration {
        if let Ok(last_accessed) = self.last_accessed.lock() {
            last_accessed.elapsed()
        } else {
            Duration::from_secs(0)
        }
    }

    /// Gets the age of this cache entry.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `CacheEntry`.
    ///
    /// # Returns
    ///
    /// A `Duration` representing the age of the entry.
    pub fn age(&self) -> Duration {
        if let Ok(cached_at) = self.cached_at.lock() {
            cached_at.elapsed()
        } else {
            Duration::from_secs(0)
        }
    }
}

/// Global cache for provider results with automatic cleanup
#[derive(Clone, Default)]
pub struct ProviderCache {
    pub cache: Arc<Mutex<HashMap<String, CacheEntry>>>,
    /// Tracks pending requests to enable request deduplication
    /// Key: cache key, Value: number of components waiting for this request
    pending_requests: Arc<Mutex<HashMap<String, u32>>>,
}

impl ProviderCache {
    /// Creates a new provider cache.
    ///
    /// # Returns
    ///
    /// A new `ProviderCache` instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if a request is currently pending for the given cache key
    ///
    /// # Arguments
    ///
    /// * `key` - The cache key to check
    ///
    /// # Returns
    ///
    /// `true` if a request is pending, `false` otherwise
    pub fn is_request_pending(&self, key: &str) -> bool {
        if let Ok(pending) = self.pending_requests.lock() {
            pending.contains_key(key)
        } else {
            false
        }
    }

    /// Mark a request as pending for the given cache key
    ///
    /// # Arguments
    ///
    /// * `key` - The cache key
    ///
    /// # Returns
    ///
    /// `true` if this is a new pending request (first component), `false` if already pending
    pub fn mark_request_pending(&self, key: &str) -> bool {
        if let Ok(mut pending) = self.pending_requests.lock() {
            let count = pending.entry(key.to_string()).or_insert(0);
            *count += 1;
            *count == 1 // Return true if this is the first component waiting
        } else {
            false
        }
    }

    /// Mark a request as no longer pending for the given cache key
    ///
    /// This should be called when the request completes. All waiting components
    /// will be notified via the refresh mechanism, so we remove the entry entirely.
    ///
    /// # Arguments
    ///
    /// * `key` - The cache key
    pub fn mark_request_complete(&self, key: &str) {
        if let Ok(mut pending) = self.pending_requests.lock() {
            if pending.remove(key).is_some() {
                crate::debug_log!("✅ [REQUEST-DEDUP] Request completed for key: {}", key);
            }
        }
    }

    /// Get the number of components waiting for a pending request
    ///
    /// # Arguments
    ///
    /// * `key` - The cache key
    ///
    /// # Returns
    ///
    /// The number of components waiting, or 0 if not pending
    pub fn pending_request_count(&self, key: &str) -> u32 {
        if let Ok(pending) = self.pending_requests.lock() {
            *pending.get(key).unwrap_or(&0)
        } else {
            0
        }
    }

    /// Retrieves a cached result by key.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `ProviderCache`.
    /// * `key` - The key to retrieve.
    ///
    /// # Returns
    ///
    /// An `Option<T>` containing the cached data if available, or `None` if not found.
    ///
    /// # Side Effects
    ///
    /// None.
    pub fn get<T: Clone + Send + Sync + 'static>(&self, key: &str) -> Option<T> {
        self.cache.lock().ok()?.get(key)?.get::<T>()
    }

    /// Retrieves a cached result with configurable options
    ///
    /// This unified method handles expiration, staleness checking, and other cache retrieval options.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `ProviderCache`.
    /// * `key` - The key to retrieve.
    /// * `options` - Cache retrieval options (expiration, stale time, etc.)
    ///
    /// # Returns
    ///
    /// An `Option<CacheGetResult<T>>` containing the cached data and staleness info if available.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use dioxus_provider::cache::{ProviderCache, CacheGetOptions};
    /// use std::time::Duration;
    ///
    /// let cache = ProviderCache::new();
    /// let options = CacheGetOptions::new()
    ///     .with_expiration(Duration::from_secs(300))
    ///     .with_stale_time(Duration::from_secs(60));
    ///
    /// if let Some(result) = cache.get_with_options::<String>("my_key", options) {
    ///     println!("Data: {}, Stale: {}", result.data, result.is_stale);
    /// }
    /// ```
    pub fn get_with_options<T: Clone + Send + Sync + 'static>(
        &self,
        key: &str,
        options: CacheGetOptions,
    ) -> Option<CacheGetResult<T>> {
        let cache_guard = self.cache.lock().ok()?;
        let entry = cache_guard.get(key)?;

        // Check expiration first
        if let Some(exp_duration) = options.expiration {
            if entry.is_expired(exp_duration) {
                drop(cache_guard);
                // Remove expired entry
                if let Ok(mut cache) = self.cache.lock() {
                    cache.remove(key);
                    crate::debug_log!(
                        "🗑️ [CACHE-EXPIRATION] Removing expired cache entry for key: {}",
                        key
                    );
                }
                return None;
            }
        }

        // Get the data
        let data = entry.get::<T>()?;

        // Check staleness if requested
        let is_stale = if options.check_staleness {
            if let Some(stale_duration) = options.stale_time {
                entry.is_stale(stale_duration)
            } else {
                false
            }
        } else {
            false
        };

        Some(CacheGetResult { data, is_stale })
    }

    /// Retrieves a cached result by key, checking for expiration with a specific expiration duration.
    ///
    /// # Deprecated
    /// Use `get_with_options()` instead for more flexible cache retrieval.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `ProviderCache`.
    /// * `key` - The key to retrieve.
    /// * `expiration` - An optional duration after which the entry is considered expired.
    ///
    /// # Returns
    ///
    /// An `Option<T>` containing the cached data if available and not expired, or `None` if expired.
    ///
    /// # Side Effects
    ///
    /// If expired, the entry is removed from the cache.
    #[deprecated(
        since = "0.1.0",
        note = "Use get_with_options() instead for more flexible cache retrieval"
    )]
    pub fn get_with_expiration<T: Clone + Send + Sync + 'static>(
        &self,
        key: &str,
        expiration: Option<Duration>,
    ) -> Option<T> {
        // First, check if the entry exists and is expired
        let is_expired = {
            let cache_guard = self.cache.lock().ok()?;
            let entry = cache_guard.get(key)?;

            if let Some(exp_duration) = expiration {
                entry.is_expired(exp_duration)
            } else {
                false
            }
        };

        // If expired, remove the entry
        if is_expired {
            if let Ok(mut cache) = self.cache.lock() {
                cache.remove(key);
                crate::debug_log!(
                    "🗑️ [CACHE-EXPIRATION] Removing expired cache entry for key: {}",
                    key
                );
            }
            return None;
        }

        // Entry is not expired, return the data
        let cache_guard = self.cache.lock().ok()?;
        let entry = cache_guard.get(key)?;
        entry.get::<T>()
    }

    /// Retrieves cached data with staleness information for SWR behavior.
    ///
    /// # Deprecated
    /// Use `get_with_options()` instead for more flexible cache retrieval.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `ProviderCache`.
    /// * `key` - The key to retrieve.
    /// * `stale_time` - An optional duration after which the entry is considered stale.
    /// * `expiration` - An optional duration after which the entry is considered expired.
    ///
    /// # Returns
    ///
    /// An `Option<(T, bool)>` containing the cached data and a boolean indicating staleness.
    ///
    /// # Side Effects
    ///
    /// None.
    #[deprecated(
        since = "0.1.0",
        note = "Use get_with_options() instead for more flexible cache retrieval"
    )]
    pub fn get_with_staleness<T: Clone + Send + Sync + 'static>(
        &self,
        key: &str,
        stale_time: Option<Duration>,
        expiration: Option<Duration>,
    ) -> Option<(T, bool)> {
        let cache_guard = self.cache.lock().ok()?;
        let entry = cache_guard.get(key)?;

        // Check if expired first
        if let Some(exp_duration) = expiration
            && entry.is_expired(exp_duration)
        {
            return None;
        }

        // Get the data
        let data = entry.get::<T>()?;

        // Check if stale
        let is_stale = if let Some(stale_duration) = stale_time {
            entry.is_stale(stale_duration)
        } else {
            false
        };

        Some((data, is_stale))
    }

    /// Sets a value for a given key.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `ProviderCache`.
    /// * `key` - The key to set.
    /// * `value` - The value to set.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the value was updated (true) or unchanged (false).
    ///
    /// # Side Effects
    ///
    /// Updates the `cached_at` timestamp if the value was updated.
    pub fn set<T: Clone + Send + Sync + PartialEq + 'static>(&self, key: String, value: T) -> bool {
        if let Ok(mut cache) = self.cache.lock() {
            if let Some(existing_entry) = cache.get_mut(&key)
                && let Some(existing_value) = existing_entry.get::<T>()
                && existing_value == value
            {
                existing_entry.refresh_timestamp();
                crate::debug_log!(
                    "⏸️ [CACHE-STORE] Value unchanged for key: {}, refreshing timestamp",
                    key
                );
                return false;
            }
            cache.insert(key.clone(), CacheEntry::new(value));
            crate::debug_log!("📊 [CACHE-STORE] Stored data for key: {}", key);
            return true;
        }
        false
    }

    /// Removes a cached result by key.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `ProviderCache`.
    /// * `key` - The key to remove.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the entry was removed.
    ///
    /// # Side Effects
    ///
    /// None.
    pub fn remove(&self, key: &str) -> bool {
        if let Ok(mut cache) = self.cache.lock() {
            cache.remove(key).is_some()
        } else {
            false
        }
    }

    /// Invalidates a cached result by key (alias for remove).
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `ProviderCache`.
    /// * `key` - The key to invalidate.
    ///
    /// # Side Effects
    ///
    /// The entry is removed from the cache.
    pub fn invalidate(&self, key: &str) {
        self.remove(key);
        crate::debug_log!(
            "🗑️ [CACHE-INVALIDATE] Invalidated cache entry for key: {}",
            key
        );
    }

    /// Clears all cached results.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `ProviderCache`.
    ///
    /// # Side Effects
    ///
    /// All entries are removed from the cache.
    pub fn clear(&self) {
        if let Ok(mut cache) = self.cache.lock() {
            #[cfg(feature = "tracing")]
            let count = cache.len();
            cache.clear();
            #[cfg(feature = "tracing")]
            crate::debug_log!("🗑️ [CACHE-CLEAR] Cleared {} cache entries", count);
        }
    }

    /// Gets the number of cached entries.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `ProviderCache`.
    ///
    /// # Returns
    ///
    /// The number of cached entries as a `usize`.
    ///
    /// # Side Effects
    ///
    /// None.
    pub fn size(&self) -> usize {
        self.cache.lock().map(|cache| cache.len()).unwrap_or(0)
    }

    /// Cleans up unused entries based on access time.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `ProviderCache`.
    /// * `unused_threshold` - The duration after which an entry is considered unused.
    ///
    /// # Returns
    ///
    /// The number of unused entries removed.
    ///
    /// # Side Effects
    ///
    /// Unused entries are removed from the cache.
    pub fn cleanup_unused_entries(&self, unused_threshold: Duration) -> usize {
        if let Ok(mut cache) = self.cache.lock() {
            let initial_size = cache.len();
            cache.retain(|_key, entry| {
                let should_keep = !entry.is_unused_for(unused_threshold);
                #[cfg(feature = "tracing")]
                if !should_keep {
                    crate::debug_log!("🧹 [CACHE-CLEANUP] Removing unused entry: {}", _key);
                }
                should_keep
            });
            let removed = initial_size - cache.len();
            if removed > 0 {
                crate::debug_log!("🧹 [CACHE-CLEANUP] Removed {} unused entries", removed);
            }
            removed
        } else {
            0
        }
    }

    /// Evicts least recently used entries to maintain cache size limit.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `ProviderCache`.
    /// * `max_size` - The maximum number of entries to keep.
    ///
    /// # Returns
    ///
    /// The number of entries evicted.
    ///
    /// # Side Effects
    ///
    /// Least recently used entries are removed from the cache.
    pub fn evict_lru_entries(&self, max_size: usize) -> usize {
        if let Ok(mut cache) = self.cache.lock() {
            if cache.len() <= max_size {
                return 0;
            }

            // Convert to vector for sorting
            let mut entries: Vec<_> = cache.drain().collect();

            // Sort by last access time (oldest first)
            entries.sort_by(|(_, a), (_, b)| {
                a.time_since_last_access().cmp(&b.time_since_last_access())
            });

            // Keep the most recently used entries
            let to_keep = entries.split_off(entries.len().saturating_sub(max_size));
            let evicted = entries.len();

            // Rebuild cache with kept entries
            cache.extend(to_keep);

            if evicted > 0 {
                crate::debug_log!(
                    "🗑️ [LRU-EVICT] Evicted {} entries due to cache size limit",
                    evicted
                );
            }
            evicted
        } else {
            0
        }
    }

    /// Performs comprehensive cache maintenance.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `ProviderCache`.
    ///
    /// # Returns
    ///
    /// A `CacheMaintenanceStats` containing statistics about the maintenance.
    ///
    /// # Side Effects
    ///
    /// Unused entries are removed and LRU entries are evicted.
    pub fn maintain(&self) -> CacheMaintenanceStats {
        CacheMaintenanceStats {
            unused_removed: self.cleanup_unused_entries(DEFAULT_UNUSED_THRESHOLD),
            lru_evicted: self.evict_lru_entries(DEFAULT_MAX_CACHE_SIZE),
            final_size: self.size(),
        }
    }

    /// Gets cache statistics.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `ProviderCache`.
    ///
    /// # Returns
    ///
    /// A `CacheStats` containing statistics about the cache.
    ///
    /// # Side Effects
    ///
    /// None.
    pub fn stats(&self) -> CacheStats {
        if let Ok(cache) = self.cache.lock() {
            let mut total_age = Duration::ZERO;
            let mut total_accesses = 0;

            for entry in cache.values() {
                total_age += entry.age();
                total_accesses += entry.access_count();
            }

            let entry_count = cache.len();
            let avg_age = if entry_count > 0 {
                total_age / entry_count as u32
            } else {
                Duration::ZERO
            };

            CacheStats {
                entry_count,
                total_accesses,
                total_references: 0, // No longer tracking references
                avg_age,
                total_size_bytes: entry_count * 1024, // Rough estimate
            }
        } else {
            CacheStats::default()
        }
    }
}

/// Statistics for cache maintenance operations
#[derive(Debug, Clone, Default)]
pub struct CacheMaintenanceStats {
    pub unused_removed: usize,
    pub lru_evicted: usize,
    pub final_size: usize,
}

/// General cache statistics
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub entry_count: usize,
    pub total_accesses: u32,
    pub total_references: u32,
    pub avg_age: Duration,
    pub total_size_bytes: usize,
}

impl CacheStats {
    pub fn avg_accesses_per_entry(&self) -> f64 {
        if self.entry_count > 0 {
            self.total_accesses as f64 / self.entry_count as f64
        } else {
            0.0
        }
    }

    pub fn avg_references_per_entry(&self) -> f64 {
        if self.entry_count > 0 {
            self.total_references as f64 / self.entry_count as f64
        } else {
            0.0
        }
    }
}
