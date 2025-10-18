//! # Cross-Platform Abstractions
//!
//! This module provides unified abstractions for cross-platform functionality,
//! eliminating code duplication between web and desktop targets.

use std::time::Duration;

// Cross-platform time imports
#[cfg(not(target_family = "wasm"))]
use std::time::{SystemTime, UNIX_EPOCH};
#[cfg(target_family = "wasm")]
use web_time::{SystemTime, UNIX_EPOCH};

// Cross-platform sleep function
#[cfg(not(target_family = "wasm"))]
use tokio::time::sleep as tokio_sleep;
#[cfg(target_family = "wasm")]
use wasmtimer::tokio::sleep as wasm_sleep;

// Cross-platform task spawning
use dioxus::prelude::spawn as dioxus_spawn;

/// Cross-platform time utilities
pub mod time {
    use super::*;

    /// Get current timestamp in seconds since Unix epoch
    pub fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    /// Sleep for the specified duration
    pub async fn sleep(duration: Duration) {
        #[cfg(not(target_family = "wasm"))]
        tokio_sleep(duration).await;
        #[cfg(target_family = "wasm")]
        wasm_sleep(duration).await;
    }

    /// Format timestamp as relative time (e.g., "5s ago", "2m ago")
    pub fn format_relative_time(timestamp: u64) -> String {
        let now = now_secs();
        let diff = now.saturating_sub(timestamp);

        if diff < 60 {
            format!("{diff}s ago")
        } else if diff < 3600 {
            format!("{}m ago", diff / 60)
        } else {
            format!("{}h ago", diff / 3600)
        }
    }
}

/// Cross-platform task management
pub mod task {
    use super::*;

    /// Spawn an async task that works on both web and desktop
    pub fn spawn<F>(future: F)
    where
        F: std::future::Future<Output = ()> + 'static,
    {
        dioxus_spawn(future);
    }

    /// Spawn a task with a name for debugging
    pub fn spawn_named<F>(#[allow(unused_variables)] name: &'static str, future: F)
    where
        F: std::future::Future<Output = ()> + 'static,
    {
        dioxus_spawn(async move {
            crate::debug_log!("Starting task: {}", name);
            future.await;
            crate::debug_log!("Completed task: {}", name);
        });
    }
}

/// Cross-platform configuration
pub mod config {
    use super::*;

    /// Default cache cleanup interval
    pub const DEFAULT_CLEANUP_INTERVAL: Duration = Duration::from_secs(30);

    /// Default cache size limit
    pub const DEFAULT_MAX_CACHE_SIZE: usize = 1000;

    /// Default unused entry threshold
    pub const DEFAULT_UNUSED_THRESHOLD: Duration = Duration::from_secs(300);
}

pub use config::*;
/// Re-export commonly used platform functions
pub use time::{format_relative_time, now_secs, sleep};
