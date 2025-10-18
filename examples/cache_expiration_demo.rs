//! # Cache Expiration Demo
//!
//! This example demonstrates cache expiration functionality in dioxus-provider.
//! Unlike SWR which serves stale data while revalidating, cache expiration
//! completely removes expired data and shows loading state during refetch.
//!
//! **Updated to use Global Providers**: This example now uses the new global provider
//! system for simplified setup. No RiverpodProvider wrapper component needed!
//!
//! ## Key Features Demonstrated:
//! - Traditional cache expiration with configurable times
//! - Loading states when cache expires
//! - Different expiration times for different data types
//! - Manual cache invalidation
//! - Cache hit/miss indicators
//! - Global provider management (NEW!)

use dioxus::prelude::*;
use dioxus_provider::hooks::ProviderState;
use dioxus_provider::{global::init_global_providers, prelude::*};
use std::{
    sync::atomic::{AtomicU32, Ordering},
    time::Duration,
};

// Cross-platform time imports
#[cfg(not(target_family = "wasm"))]
use std::time::{SystemTime, UNIX_EPOCH};
#[cfg(target_family = "wasm")]
use web_time::{SystemTime, UNIX_EPOCH};

// Cross-platform sleep function
#[cfg(not(target_family = "wasm"))]
use tokio::time::sleep;
#[cfg(target_family = "wasm")]
use wasmtimer::tokio::sleep;

/// Global counter for tracking API calls
static API_CALL_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Quick expiring cache - expires after 5 seconds
#[provider(cache_expiration = "5s")]
async fn fetch_quick_expiring_data() -> Result<QuickData, String> {
    println!("🔄 [FETCH] Quick data fetch started");
    sleep(Duration::from_millis(1000)).await;

    let call_number = API_CALL_COUNTER.fetch_add(1, Ordering::SeqCst) + 1;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    println!("✅ [FETCH] Quick data fetch completed - call #{call_number}");
    Ok(QuickData {
        message: format!("Quick data call #{call_number}"),
        fetched_at: timestamp,
        expires_in: 5,
    })
}

/// Medium expiring cache - expires after 15 seconds  
#[provider(cache_expiration = "15s")]
async fn fetch_medium_expiring_data() -> Result<MediumData, String> {
    sleep(Duration::from_millis(2000)).await;

    let call_number = API_CALL_COUNTER.fetch_add(1, Ordering::SeqCst) + 1;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    Ok(MediumData {
        content: format!("Medium-lived content #{call_number}"),
        details: vec![
            "This data is cached for 15 seconds".to_string(),
            "After expiration, shows loading state".to_string(),
            format!("Fetched at timestamp: {}", timestamp),
        ],
        fetched_at: timestamp,
        expires_in: 15,
    })
}

/// Long expiring cache - expires after 30 seconds
#[provider(cache_expiration = "30s")]
async fn fetch_long_expiring_data() -> Result<LongData, String> {
    sleep(Duration::from_millis(1500)).await;

    let call_number = API_CALL_COUNTER.fetch_add(1, Ordering::SeqCst) + 1;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Simulate occasional failures
    if call_number % 5 == 0 {
        return Err("Simulated server error for demo".to_string());
    }

    Ok(LongData {
        title: format!("Long-lived data #{call_number}"),
        description: "This expensive operation is cached for 30 seconds".to_string(),
        metadata: DataMetadata {
            size_mb: (call_number * 2) % 100,
            processing_time_ms: 1500,
            version: format!("v{}.{}", call_number / 10, call_number % 10),
        },
        fetched_at: timestamp,
        expires_in: 30,
    })
}

/// Parameterized cache with expiration - user-specific data
#[provider(cache_expiration = "10s")]
async fn fetch_user_cache_data(user_id: u32) -> Result<UserCacheData, String> {
    sleep(Duration::from_millis(800)).await;

    let call_number = API_CALL_COUNTER.fetch_add(1, Ordering::SeqCst) + 1;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    Ok(UserCacheData {
        user_id,
        preferences: UserPreferences {
            theme: if call_number % 2 == 0 {
                "dark"
            } else {
                "light"
            }
            .to_string(),
            language: "en".to_string(),
            notifications: call_number % 3 == 0,
        },
        activity_summary: format!("User {user_id} activity summary (call #{call_number})"),
        last_login: timestamp - (call_number as u64 * 3600), // Simulate different login times
        fetched_at: timestamp,
        expires_in: 10,
    })
}

/// Data structures for our demo
#[derive(Debug, Clone, PartialEq)]
pub struct QuickData {
    message: String,
    fetched_at: u64,
    expires_in: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MediumData {
    content: String,
    details: Vec<String>,
    fetched_at: u64,
    expires_in: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LongData {
    title: String,
    description: String,
    metadata: DataMetadata,
    fetched_at: u64,
    expires_in: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DataMetadata {
    size_mb: u32,
    processing_time_ms: u32,
    version: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UserCacheData {
    user_id: u32,
    preferences: UserPreferences,
    activity_summary: String,
    last_login: u64,
    fetched_at: u64,
    expires_in: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UserPreferences {
    theme: String,
    language: String,
    notifications: bool,
}

/// Main cache expiration demo component
#[component]
fn CacheExpirationDemo() -> Element {
    let mut selected_user_id = use_signal(|| 1u32);

    // Add a timer to force periodic re-renders to trigger cache expiration checks
    let mut _timer_count = use_signal(|| 0u32);
    use_effect(move || {
        spawn(async move {
            loop {
                sleep(Duration::from_secs(2)).await;
                _timer_count.with_mut(|count| {
                    *count += 1;
                    println!(
                        "⏰ Timer tick #{count} - triggering re-render to check cache expiration"
                    );
                });
            }
        });
    });

    // Different cache expiration providers
    let quick_data = use_provider(fetch_quick_expiring_data(), ());
    let medium_data = use_provider(fetch_medium_expiring_data(), ());
    let long_data = use_provider(fetch_long_expiring_data(), ());
    let user_data = use_provider(fetch_user_cache_data(), (*selected_user_id.read(),));

    // Manual invalidation functions
    let invalidate_quick = use_invalidate_provider(fetch_quick_expiring_data(), ());
    let invalidate_medium = use_invalidate_provider(fetch_medium_expiring_data(), ());
    let invalidate_long = use_invalidate_provider(fetch_long_expiring_data(), ());
    let invalidate_user =
        use_invalidate_provider(fetch_user_cache_data(), *selected_user_id.read());

    rsx! {
        div { class: "cache-demo",
            header { class: "demo-header",
                h1 { "💾 Cache Expiration Demo" }
                p { class: "demo-description",
                    "Traditional cache expiration removes data after TTL and shows loading during refetch."
                }
                div { class: "cache-times-info",
                    span { class: "cache-badge quick", "Quick: 5s" }
                    span { class: "cache-badge medium", "Medium: 15s" }
                    span { class: "cache-badge long", "Long: 30s" }
                    span { class: "cache-badge user", "User: 10s" }
                }
            }

            div { class: "demo-controls",
                h3 { "Manual Cache Invalidation:" }
                div { class: "control-group",
                    button {
                        class: "invalidate-btn quick",
                        onclick: move |_| invalidate_quick(),
                        "💥 Clear Quick Cache"
                    }
                    button {
                        class: "invalidate-btn medium",
                        onclick: move |_| invalidate_medium(),
                        "💥 Clear Medium Cache"
                    }
                    button {
                        class: "invalidate-btn long",
                        onclick: move |_| invalidate_long(),
                        "💥 Clear Long Cache"
                    }
                    button {
                        class: "invalidate-btn user",
                        onclick: move |_| invalidate_user(),
                        "💥 Clear User Cache"
                    }
                }
                div { class: "user-selector",
                    label { "User ID for parameterized cache: " }
                    input {
                        r#type: "number",
                        value: "{selected_user_id}",
                        min: "1",
                        max: "10",
                        oninput: move |e| {
                            if let Ok(id) = e.value().parse::<u32>() {
                                selected_user_id.set(id);
                            }
                        },
                    }
                }
            }

            div { class: "cache-grid",
                // Quick expiring cache (5s)
                CacheCard {
                    title: "⚡ Quick Cache (5s TTL)",
                    cache_type: "quick",
                    data: quick_data,
                    render_success: |data: &QuickData| rsx! {
                        div { class: "data-content",
                            h4 { "{data.message}" }
                            CacheTimestamp { fetched_at: data.fetched_at, expires_in: data.expires_in }
                        }
                    },
                }

                // Medium expiring cache (15s)
                CacheCard {
                    title: "🔄 Medium Cache (15s TTL)",
                    cache_type: "medium",
                    data: medium_data,
                    render_success: |data: &MediumData| rsx! {
                        div { class: "data-content",
                            h4 { "{data.content}" }
                            ul { class: "details-list",
                                for detail in &data.details {
                                    li { "{detail}" }
                                }
                            }
                            CacheTimestamp { fetched_at: data.fetched_at, expires_in: data.expires_in }
                        }
                    },
                }

                // Long expiring cache (30s)
                CacheCard {
                    title: "🐌 Long Cache (30s TTL)",
                    cache_type: "long",
                    data: long_data,
                    render_success: |data: &LongData| rsx! {
                        div { class: "data-content",
                            h4 { "{data.title}" }
                            p { "{data.description}" }
                            div { class: "metadata",
                                p { "📦 Size: {data.metadata.size_mb} MB" }
                                p { "⏱️ Processing: {data.metadata.processing_time_ms}ms" }
                                p { "🏷️ Version: {data.metadata.version}" }
                            }
                            CacheTimestamp { fetched_at: data.fetched_at, expires_in: data.expires_in }
                        }
                    },
                }

                // User-specific cache (10s)
                CacheCard {
                    title: format!("👤 User Cache (10s TTL) - User {}", *selected_user_id.read()),
                    cache_type: "user",
                    data: user_data,
                    render_success: |data: &UserCacheData| rsx! {
                        div { class: "data-content",
                            div { class: "user-preferences",
                                h5 { "User Preferences:" }
                                p { "🎨 Theme: {data.preferences.theme}" }
                                p { "🌍 Language: {data.preferences.language}" }
                                p { "🔔 Notifications: {data.preferences.notifications}" }
                            }
                            p { class: "activity", "{data.activity_summary}" }
                            p { class: "last-login", "Last login: {format_timestamp(data.last_login)}" }
                            CacheTimestamp { fetched_at: data.fetched_at, expires_in: data.expires_in }
                        }
                    },
                }
            }

            div { class: "behavior-explanation",
                h3 { "📋 Cache Expiration vs SWR Comparison" }
                div { class: "comparison-table",
                    div { class: "comparison-row header",
                        div { "Aspect" }
                        div { "Cache Expiration" }
                        div { "SWR (Stale-While-Revalidate)" }
                    }
                    div { class: "comparison-row",
                        div { "Data Availability" }
                        div { "Gone after expiration" }
                        div { "Always available (stale or fresh)" }
                    }
                    div { class: "comparison-row",
                        div { "Loading State" }
                        div { "Shows on cache miss" }
                        div { "Only on initial load" }
                    }
                    div { class: "comparison-row",
                        div { "User Experience" }
                        div { "Brief loading delays" }
                        div { "Instant response" }
                    }
                    div { class: "comparison-row",
                        div { "Use Case" }
                        div { "Fresh data required" }
                        div { "Stale data acceptable" }
                    }
                }
            }

            footer { class: "demo-footer",
                p { class: "instructions",
                    "💡 Watch cache expirations: Quick (5s) expires first, then User (10s), Medium (15s), and Long (30s)!"
                }
            }
        }

        style { {include_str!("./assets/cache_expiration_styles.css")} }
    }
}

/// Reusable cache card component
#[component]
fn CacheCard<T: 'static + Clone + PartialEq, E: 'static + Clone + PartialEq + std::fmt::Display>(
    title: String,
    cache_type: &'static str,
    data: Signal<ProviderState<T, E>>,
    render_success: fn(&T) -> Element,
) -> Element {
    let cache_status = match &*data.read() {
        ProviderState::Loading { .. } => "cache-miss",
        ProviderState::Success(_) => "cache-hit",
        ProviderState::Error(_) => "cache-error",
    };

    rsx! {
        div { class: format!("cache-card {}", cache_type),
            div { class: "card-header",
                h3 { "{title}" }
                div { class: format!("cache-status {}", cache_status),
                    match cache_status {
                        "cache-hit" => "✅ CACHED",
                        "cache-miss" => "🔄 LOADING",
                        "cache-error" => "❌ ERROR",
                        _ => "",
                    }
                }
            }
            div { class: "card-content",
                match &*data.read() {
                    ProviderState::Loading { .. } => rsx! {
                        div { class: "loading-container",
                            div { class: "loading-spinner" }
                            span { "Cache expired - fetching fresh data..." }
                        }
                    },
                    ProviderState::Error(e) => rsx! {
                        div { class: "error-container",
                            span { class: "error-icon", "❌" }
                            span { class: "error-message", "Error: {e}" }
                        }
                    },
                    ProviderState::Success(value) => render_success(value),
                }
            }
        }
    }
}

/// Component to display cache timing information
#[component]
fn CacheTimestamp(fetched_at: u64, expires_in: u32) -> Element {
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let age = current_time.saturating_sub(fetched_at);
    let time_until_expiry = (expires_in as u64).saturating_sub(age);

    rsx! {
        div { class: "cache-timing",
            p { class: "fetched-time", "⏰ Fetched: {format_timestamp(fetched_at)}" }
            p { class: "cache-age", "📅 Age: {age}s" }
            p { class: if time_until_expiry > 0 { "expires-in" } else { "expired" },
                if time_until_expiry > 0 {
                    "⏳ Expires in: {time_until_expiry}s"
                } else {
                    "💥 EXPIRED"
                }
            }
        }
    }
}

/// Helper function to format timestamps
fn format_timestamp(timestamp: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let diff = now.saturating_sub(timestamp);

    if diff < 60 {
        format!("{diff}s ago")
    } else if diff < 3600 {
        format!("{}m ago", diff / 60)
    } else {
        format!("{}h ago", diff / 3600)
    }
}

/// Application root - now using global providers for simplified setup
fn app() -> Element {
    rsx! {
        CacheExpirationDemo {}
    }
}

fn main() {
    // Initialize tracing for debug logs
    tracing_subscriber::fmt::init();

    // Initialize global providers at application startup
    dioxus_provider::init();

    launch(app);
}
