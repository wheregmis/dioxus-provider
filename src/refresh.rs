//! # Refresh Registry
//!
//! This module provides the refresh registry that manages reactive updates and interval tasks
//! for providers. It handles triggering re-execution of providers when their dependencies change
//! and manages automatic refresh intervals for live data providers.
//!
//! ## Key Features
//!
//! - **Refresh Tracking**: Maintains counters for provider refresh events
//! - **Reactive Context Management**: Subscribes and notifies reactive contexts when data changes
//! - **Interval Tasks**: Manages background tasks for auto-refreshing providers
//! - **Revalidation Control**: Prevents duplicate revalidations and manages ongoing operations
//!
//! ## Cross-Platform Compatibility
//!
//! This module uses cross-platform abstractions:
//! - `dioxus::spawn` for background tasks (works on both web and desktop)
//! - `wasmtimer` for web timing and `tokio` for desktop timing
//! - Automatic task cleanup when components unmount

use dioxus::{core::ReactiveContext, prelude::*};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, atomic::AtomicBool},
    time::Duration,
};

#[cfg(not(target_family = "wasm"))]
use tokio::time;
#[cfg(target_family = "wasm")]
use wasmtimer::tokio as time;

/// Type alias for reactive context storage
type ReactiveContextSet = Arc<Mutex<HashSet<ReactiveContext>>>;
type ReactiveContextRegistry = Arc<Mutex<HashMap<String, ReactiveContextSet>>>;

/// Task type for different periodic operations
#[derive(Debug, Clone, PartialEq)]
pub enum TaskType {
    /// Interval refresh task that re-executes providers at regular intervals
    IntervalRefresh,
    /// Stale checking task that monitors for stale data and triggers revalidation
    StaleCheck,
    /// Cache cleanup task that removes unused entries and enforces size limits
    CacheCleanup,
    /// Cache expiration task that monitors and removes expired entries
    CacheExpiration,
}

/// Registry for periodic tasks (intervals and stale checks)
/// Stores task type, duration, and cancellation flag
type PeriodicTaskRegistry = Arc<Mutex<HashMap<String, (TaskType, Duration, Arc<AtomicBool>)>>>;

/// Global registry for refresh signals that can trigger provider re-execution
///
/// The `RefreshRegistry` manages the reactive update system for providers. It tracks
/// which reactive contexts are subscribed to which providers, maintains refresh counters,
/// and manages periodic tasks for both auto-refreshing and stale-checking.
///
/// ## Thread Safety
///
/// All internal state is protected by mutexes to ensure thread-safe access across
/// different contexts and background tasks.
#[derive(Clone, Default)]
pub struct RefreshRegistry {
    /// Counters for tracking how many times each provider has been refreshed
    refresh_counters: Arc<Mutex<HashMap<String, u64>>>,
    /// Registry of reactive contexts subscribed to each provider key
    reactive_contexts: ReactiveContextRegistry,
    /// Registry of periodic tasks (both interval refresh and stale checking)
    periodic_tasks: PeriodicTaskRegistry,
    /// Set of provider keys that are currently being revalidated
    ongoing_revalidations: Arc<Mutex<HashSet<String>>>,
}

impl RefreshRegistry {
    /// Create a new refresh registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the current refresh count for a provider key
    ///
    /// Returns the number of times the provider has been refreshed, or 0 if not found.
    pub fn get_refresh_count(&self, key: &str) -> u64 {
        if let Ok(counters) = self.refresh_counters.lock() {
            *counters.get(key).unwrap_or(&0)
        } else {
            0
        }
    }

    /// Subscribe a reactive context to refresh events for a provider key
    ///
    /// When the provider is refreshed, the reactive context will be marked as dirty,
    /// causing any components using it to re-render.
    pub fn subscribe_to_refresh(&self, key: &str, reactive_context: ReactiveContext) {
        if let Ok(mut contexts) = self.reactive_contexts.lock() {
            let key_contexts = contexts
                .entry(key.to_string())
                .or_insert_with(|| Arc::new(Mutex::new(HashSet::new())));
            if let Ok(mut context_set) = key_contexts.lock() {
                context_set.insert(reactive_context);
            }
        }
    }

    /// Trigger a refresh for a provider key
    ///
    /// This increments the refresh counter and marks all subscribed reactive contexts
    /// as dirty, causing components to re-render and providers to re-execute.
    pub fn trigger_refresh(&self, key: &str) {
        // Increment the counter
        if let Ok(mut counters) = self.refresh_counters.lock() {
            let counter = counters.entry(key.to_string()).or_insert(0);
            *counter += 1;
        }

        // Mark all reactive contexts as dirty
        if let Ok(contexts) = self.reactive_contexts.lock() {
            if let Some(key_contexts) = contexts.get(key) {
                if let Ok(context_set) = key_contexts.lock() {
                    for reactive_context in context_set.iter() {
                        reactive_context.mark_dirty();
                    }
                }
            }
        }
    }

    /// Clear all cached data and trigger refresh for all providers
    ///
    /// This is useful for global cache invalidation scenarios.
    pub fn clear_all(&self) {
        if let Ok(counters) = self.refresh_counters.lock() {
            let keys: Vec<String> = counters.keys().cloned().collect();
            drop(counters);

            for key in keys {
                self.trigger_refresh(&key);
            }
        }
    }

    /// Start a periodic task for automatic provider operations
    ///
    /// Creates a background task that will call the provided function at regular
    /// intervals. Supports both interval refresh and stale checking operations.
    /// If an existing task exists with a longer interval, it will be replaced.
    /// Tasks with shorter intervals are preserved to avoid unnecessary re-creation.
    ///
    /// ## Cross-Platform Implementation
    ///
    /// Uses `dioxus::spawn` to create tasks that work on both web and desktop platforms.
    /// Tasks can be cancelled explicitly using stop_* methods, which set a cancellation flag.
    pub fn start_periodic_task<F>(
        &self,
        key: &str,
        task_type: TaskType,
        interval: Duration,
        task_fn: F,
    ) where
        F: Fn() + Send + 'static,
    {
        if let Ok(mut tasks) = self.periodic_tasks.lock() {
            let task_key = format!("{key}:{task_type:?}");

            // For certain task types, don't create multiple tasks for the same provider
            if (task_type == TaskType::StaleCheck || task_type == TaskType::CacheExpiration)
                && tasks
                    .iter()
                    .any(|(k, (t, _, _))| k.starts_with(&format!("{key}:")) && *t == task_type)
            {
                return;
            }

            // Cancel existing task if it exists and the new interval is shorter (for interval tasks)
            let should_create_new_task = match tasks.get(&task_key) {
                None => true,
                Some((_, current_interval, cancel_flag)) => {
                    if task_type == TaskType::IntervalRefresh && interval < *current_interval {
                        // Signal existing task to stop
                        cancel_flag.store(true, std::sync::atomic::Ordering::SeqCst);
                        tasks.remove(&task_key);
                        true
                    } else {
                        false // Don't replace stale check or cache expiration tasks
                    }
                }
            };

            if should_create_new_task {
                // Adjust interval for different task types
                let actual_interval = match task_type {
                    TaskType::StaleCheck => Duration::max(
                        Duration::min(interval / 4, Duration::from_secs(30)),
                        Duration::from_secs(1),
                    ),
                    TaskType::CacheExpiration => Duration::max(
                        Duration::min(interval / 4, Duration::from_secs(30)),
                        Duration::from_secs(1),
                    ),
                    _ => interval,
                };

                // Create cancellation flag for this task
                let cancel_flag = Arc::new(AtomicBool::new(false));
                let cancel_flag_clone = cancel_flag.clone();
                let task_fn = Arc::new(task_fn);

                spawn(async move {
                    loop {
                        // Check if task should be cancelled before sleeping
                        if cancel_flag_clone.load(std::sync::atomic::Ordering::SeqCst) {
                            break;
                        }
                        
                        time::sleep(actual_interval).await;
                        
                        // Check if task should be cancelled before running
                        if cancel_flag_clone.load(std::sync::atomic::Ordering::SeqCst) {
                            break;
                        }
                        
                        task_fn();
                    }
                });

                tasks.insert(task_key, (task_type, interval, cancel_flag));
            }
        }
    }

    /// Start an interval task for automatic provider refresh
    ///
    /// This is a convenience method for starting interval refresh tasks.
    pub fn start_interval_task<F>(&self, key: &str, interval: Duration, refresh_fn: F)
    where
        F: Fn() + Send + 'static,
    {
        self.start_periodic_task(key, TaskType::IntervalRefresh, interval, refresh_fn);
    }

    /// Start a stale check task for SWR behavior
    ///
    /// This is a convenience method for starting stale checking tasks.
    pub fn start_stale_check_task<F>(&self, key: &str, stale_time: Duration, stale_check_fn: F)
    where
        F: Fn() + Send + 'static,
    {
        self.start_periodic_task(key, TaskType::StaleCheck, stale_time, stale_check_fn);
    }

    /// Stop a periodic task
    ///
    /// Signals the task to stop by setting its cancellation flag and removes it from the registry.
    /// The task will stop after its current iteration completes.
    pub fn stop_periodic_task(&self, key: &str, task_type: TaskType) {
        if let Ok(mut tasks) = self.periodic_tasks.lock() {
            let task_key = format!("{key}:{task_type:?}");
            if let Some((_, _, cancel_flag)) = tasks.remove(&task_key) {
                // Signal the task to stop
                cancel_flag.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        }
    }

    /// Stop an interval task
    ///
    /// This is a convenience method for stopping interval refresh tasks.
    pub fn stop_interval_task(&self, key: &str) {
        self.stop_periodic_task(key, TaskType::IntervalRefresh);
    }

    /// Stop a stale check task
    ///
    /// This is a convenience method for stopping stale checking tasks.
    pub fn stop_stale_check_task(&self, key: &str) {
        self.stop_periodic_task(key, TaskType::StaleCheck);
    }

    /// Check if a revalidation is currently in progress for a provider key
    ///
    /// This prevents duplicate revalidations from being started simultaneously.
    pub fn is_revalidation_in_progress(&self, key: &str) -> bool {
        if let Ok(revalidations) = self.ongoing_revalidations.lock() {
            revalidations.contains(key)
        } else {
            false
        }
    }

    /// Start a revalidation for a provider key
    ///
    /// Returns true if the revalidation was started, false if one was already in progress.
    /// This prevents duplicate revalidations from running simultaneously.
    pub fn start_revalidation(&self, key: &str) -> bool {
        if let Ok(mut revalidations) = self.ongoing_revalidations.lock() {
            if revalidations.contains(key) {
                false
            } else {
                revalidations.insert(key.to_string());
                true
            }
        } else {
            false
        }
    }

    /// Complete a revalidation for a provider key
    ///
    /// This should be called when a revalidation finishes, regardless of success or failure.
    pub fn complete_revalidation(&self, key: &str) {
        if let Ok(mut revalidations) = self.ongoing_revalidations.lock() {
            revalidations.remove(key);
        }
    }

    /// Get statistics about the refresh registry
    pub fn stats(&self) -> RefreshRegistryStats {
        let refresh_count = if let Ok(counters) = self.refresh_counters.lock() {
            counters.len()
        } else {
            0
        };

        let context_count = if let Ok(contexts) = self.reactive_contexts.lock() {
            contexts.len()
        } else {
            0
        };

        let task_count = if let Ok(tasks) = self.periodic_tasks.lock() {
            tasks.len()
        } else {
            0
        };

        let revalidation_count = if let Ok(revalidations) = self.ongoing_revalidations.lock() {
            revalidations.len()
        } else {
            0
        };

        RefreshRegistryStats {
            refresh_count,
            context_count,
            task_count,
            revalidation_count,
        }
    }

    /// Clean up unused subscriptions and tasks
    pub fn cleanup(&self) -> RefreshCleanupStats {
        let mut stats = RefreshCleanupStats::default();

        // Clean up unused reactive contexts
        if let Ok(mut contexts) = self.reactive_contexts.lock() {
            let initial_context_count = contexts.len();
            contexts.retain(|_, context_set| {
                if let Ok(set) = context_set.lock() {
                    !set.is_empty()
                } else {
                    false
                }
            });
            stats.contexts_removed = initial_context_count - contexts.len();
        }

        // Clean up completed revalidations (should be empty, but just in case)
        if let Ok(mut revalidations) = self.ongoing_revalidations.lock() {
            stats.revalidations_cleared = revalidations.len();
            revalidations.clear();
        }

        stats
    }
}

/// Statistics for the refresh registry
#[derive(Debug, Clone, Default)]
pub struct RefreshRegistryStats {
    pub refresh_count: usize,
    pub context_count: usize,
    pub task_count: usize,
    pub revalidation_count: usize,
}

/// Statistics for refresh registry cleanup operations
#[derive(Debug, Clone, Default)]
pub struct RefreshCleanupStats {
    pub contexts_removed: usize,
    pub revalidations_cleared: usize,
}
