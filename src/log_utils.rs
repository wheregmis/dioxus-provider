//! Internal logging utilities for consistent log formatting across the library
//!
//! This module provides macros that adapt log messages based on feature flags:
//! - `tracing`: Enable/disable all logging (enabled by default)
//! - `plain-logs`: When enabled with `tracing`, uses plain text prefixes instead of emojis
//!
//! ## Usage
//!
//! ```toml
//! # Default: tracing enabled with emojis
//! dioxus-provider = "0.1"
//!
//! # Disable all logging
//! dioxus-provider = { version = "0.1", default-features = false }
//!
//! # Enable tracing with plain text (no emojis)
//! dioxus-provider = { version = "0.1", features = ["plain-logs"] }
//! ```

/// Internal debug logging macro that respects the tracing feature flag
#[macro_export]
macro_rules! debug_log {
    ($($arg:tt)*) => {
        #[cfg(feature = "tracing")]
        tracing::debug!($($arg)*);
    };
}

/// Logs a cache hit with appropriate formatting
#[macro_export]
macro_rules! log_cache_hit {
    ($($arg:tt)*) => {
        #[cfg(all(feature = "tracing", not(feature = "plain-logs")))]
        tracing::debug!("📊 [CACHE-HIT] {}", format!($($arg)*));
        #[cfg(all(feature = "tracing", feature = "plain-logs"))]
        tracing::debug!("[CACHE-HIT] {}", format!($($arg)*));
    };
}

/// Logs a cache store operation with appropriate formatting
#[macro_export]
macro_rules! log_cache_store {
    ($($arg:tt)*) => {
        #[cfg(all(feature = "tracing", not(feature = "plain-logs")))]
        tracing::debug!("📊 [CACHE-STORE] {}", format!($($arg)*));
        #[cfg(all(feature = "tracing", feature = "plain-logs"))]
        tracing::debug!("[CACHE-STORE] {}", format!($($arg)*));
    };
}

/// Logs a cache invalidation with appropriate formatting
#[macro_export]
macro_rules! log_cache_invalidate {
    ($($arg:tt)*) => {
        #[cfg(all(feature = "tracing", not(feature = "plain-logs")))]
        tracing::debug!("🗑️ [CACHE-INVALIDATE] {}", format!($($arg)*));
        #[cfg(all(feature = "tracing", feature = "plain-logs"))]
        tracing::debug!("[CACHE-INVALIDATE] {}", format!($($arg)*));
    };
}

/// Logs a mutation start with appropriate formatting
#[macro_export]
macro_rules! log_mutation_start {
    ($($arg:tt)*) => {
        #[cfg(all(feature = "tracing", not(feature = "plain-logs")))]
        tracing::debug!("🔄 [MUTATION] {}", format!($($arg)*));
        #[cfg(all(feature = "tracing", feature = "plain-logs"))]
        tracing::debug!("[MUTATION] {}", format!($($arg)*));
    };
}

/// Logs a mutation success with appropriate formatting
#[macro_export]
macro_rules! log_mutation_success {
    ($($arg:tt)*) => {
        #[cfg(all(feature = "tracing", not(feature = "plain-logs")))]
        tracing::debug!("✅ [MUTATION] {}", format!($($arg)*));
        #[cfg(all(feature = "tracing", feature = "plain-logs"))]
        tracing::debug!("[MUTATION-SUCCESS] {}", format!($($arg)*));
    };
}

/// Logs a mutation error with appropriate formatting
#[macro_export]
macro_rules! log_mutation_error {
    ($($arg:tt)*) => {
        #[cfg(all(feature = "tracing", not(feature = "plain-logs")))]
        tracing::debug!("❌ [MUTATION] {}", format!($($arg)*));
        #[cfg(all(feature = "tracing", feature = "plain-logs"))]
        tracing::debug!("[MUTATION-ERROR] {}", format!($($arg)*));
    };
}

/// Logs an optimistic update with appropriate formatting
#[macro_export]
macro_rules! log_optimistic {
    ($($arg:tt)*) => {
        #[cfg(all(feature = "tracing", not(feature = "plain-logs")))]
        tracing::debug!("⚡ [OPTIMISTIC] {}", format!($($arg)*));
        #[cfg(all(feature = "tracing", feature = "plain-logs"))]
        tracing::debug!("[OPTIMISTIC] {}", format!($($arg)*));
    };
}

/// Logs a rollback operation with appropriate formatting
#[macro_export]
macro_rules! log_rollback {
    ($($arg:tt)*) => {
        #[cfg(all(feature = "tracing", not(feature = "plain-logs")))]
        tracing::debug!("🔄 [ROLLBACK] {}", format!($($arg)*));
        #[cfg(all(feature = "tracing", feature = "plain-logs"))]
        tracing::debug!("[ROLLBACK] {}", format!($($arg)*));
    };
}
