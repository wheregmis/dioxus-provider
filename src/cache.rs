//! # Cache Management for dioxus-provider
//!
//! This module implements a global, type-erased cache for provider results, supporting:
//! - **Expiration**: Entries are removed after a configurable TTL.
//! - **Staleness (SWR)**: Entries can be marked stale and revalidated in the background.
//! - **LRU Eviction**: Least-recently-used entries are evicted to maintain a size limit.
//! - **Reference Counting**: Tracks active users of each entry for safe cleanup.
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
use tracing::debug;

use crate::platform::{DEFAULT_MAX_CACHE_SIZE, DEFAULT_UNUSED_THRESHOLD};

// Platform-specific time imports
#[cfg(not(target_family = "wasm"))]
use std::time::Instant;
#[cfg(target_family = "wasm")]
use web_time::Instant;

use dioxus_lib::prelude::Task;

/// Represents the state of an async operation
#[derive(Clone, PartialEq)]
pub enum ProviderState<T, E> {
    /// The operation is currently loading
    Loading { task: Task },
    /// The operation completed successfully with data
    Success(T),
    /// The operation failed with an error
    Error(E),
}

impl<T, E> ProviderState<T, E> {
    /// Returns true if the state is currently loading
    pub fn is_loading(&self) -> bool {
        matches!(self, ProviderState::Loading { task: _ })
    }

    /// Returns true if the state contains successful data
    pub fn is_success(&self) -> bool {
        matches!(self, ProviderState::Success(_))
    }

    /// Returns true if the state contains an error
    pub fn is_error(&self) -> bool {
        matches!(self, ProviderState::Error(_))
    }

    /// Returns the data if successful, None otherwise
    pub fn data(&self) -> Option<&T> {
        match self {
            ProviderState::Success(data) => Some(data),
            _ => None,
        }
    }

    /// Returns the error if failed, None otherwise
    pub fn error(&self) -> Option<&E> {
        match self {
            ProviderState::Error(error) => Some(error),
            _ => None,
        }
    }
}

/// A type-erased cache entry for storing provider results with timestamp and reference counting
#[derive(Clone)]
pub struct CacheEntry {
    data: Arc<dyn Any + Send + Sync>,
    cached_at: Arc<Mutex<Instant>>,
    reference_count: Arc<AtomicU32>,
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
            reference_count: Arc::new(AtomicU32::new(0)),
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

    /// Increments the reference count for the cache entry.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `CacheEntry`.
    ///
    /// # Side Effects
    ///
    /// Increments the `reference_count` by 1.
    pub fn add_reference(&self) {
        self.reference_count.fetch_add(1, Ordering::SeqCst);
    }

    /// Decrements the reference count for the cache entry.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `CacheEntry`.
    ///
    /// # Side Effects
    ///
    /// Decrements the `reference_count` by 1.
    pub fn remove_reference(&self) {
        self.reference_count.fetch_sub(1, Ordering::SeqCst);
    }

    /// Gets the current reference count for the cache entry.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `CacheEntry`.
    ///
    /// # Returns
    ///
    /// The current reference count as a `u32`.
    pub fn reference_count(&self) -> u32 {
        self.reference_count.load(Ordering::SeqCst)
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

    /// Retrieves a cached result by key, checking for expiration with a specific expiration duration.
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
                debug!(
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
    pub fn get_with_staleness<T: Clone + Send + Sync + 'static>(
        &self,
        key: &str,
        stale_time: Option<Duration>,
        expiration: Option<Duration>,
    ) -> Option<(T, bool)> {
        let cache_guard = self.cache.lock().ok()?;
        let entry = cache_guard.get(key)?;

        // Check if expired first
        if let Some(exp_duration) = expiration {
            if entry.is_expired(exp_duration) {
                return None;
            }
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
            if let Some(existing_entry) = cache.get_mut(&key) {
                if let Some(existing_value) = existing_entry.get::<T>() {
                    if existing_value == value {
                        existing_entry.refresh_timestamp();
                        debug!(
                            "⏸️ [CACHE-STORE] Value unchanged for key: {}, refreshing timestamp",
                            key
                        );
                        return false;
                    }
                }
            }
            cache.insert(key.clone(), CacheEntry::new(value));
            debug!("📊 [CACHE-STORE] Stored data for key: {}", key);
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
        debug!(
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
            let count = cache.len();
            cache.clear();
            debug!("🗑️ [CACHE-CLEAR] Cleared {} cache entries", count);
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
            cache.retain(|key, entry| {
                let should_keep =
                    !entry.is_unused_for(unused_threshold) || entry.reference_count() > 0;
                if !should_keep {
                    debug!("🧹 [CACHE-CLEANUP] Removing unused entry: {}", key);
                }
                should_keep
            });
            let removed = initial_size - cache.len();
            if removed > 0 {
                debug!("🧹 [CACHE-CLEANUP] Removed {} unused entries", removed);
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
                debug!(
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
            let mut total_references = 0;

            for entry in cache.values() {
                total_age += entry.age();
                total_accesses += entry.access_count();
                total_references += entry.reference_count();
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
                total_references,
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
