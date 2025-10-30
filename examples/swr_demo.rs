#![allow(unpredictable_function_pointer_comparisons)]
//! # SWR (Stale-While-Revalidate) Demo
//!
//! This example demonstrates the basic SWR pattern using dioxus-provider.
//! SWR serves cached (stale) data immediately while revalidating in the background,
//! providing a smooth user experience with fresh data.
//!
//! **Updated to use Global Providers**: This example now uses the new global provider
//! system for simplified setup. No RiverpodProvider wrapper component needed!
//!
//! **Reactive SWR**: The library now automatically monitors cache entries for staleness
//! and triggers background revalidation without requiring manual timers or re-renders.
//!
//! ## Key Features Demonstrated:
//! - Stale-while-revalidate pattern with configurable stale time
//! - Automatic background staleness checking (NEW!)
//! - Instant data serving from cache
//! - Background revalidation without manual intervention
//! - Manual refresh functionality
//! - Loading states and error handling
//! - Global provider management

use dioxus::prelude::*;
use dioxus_provider::prelude::*;
use std::{
    sync::atomic::{AtomicU32, Ordering},
    time::Duration,
};

// Cross-platform sleep function
#[cfg(not(target_family = "wasm"))]
use tokio::time::sleep;
#[cfg(target_family = "wasm")]
use wasmtimer::tokio::sleep;

/// Global counter to simulate different API responses
static API_CALL_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Simple SWR provider that becomes stale after 3 seconds
/// After 3 seconds, cached data is served immediately while new data is fetched in background
/// Uses intelligent cache management: expires after 30s and cleanup runs every 7.5s
#[provider(stale_time = "3s", cache_expiration = "30s")]
async fn fetch_user_profile() -> Result<UserProfile, String> {
    // Simulate API call delay
    sleep(Duration::from_millis(1500)).await;

    let call_number = API_CALL_COUNTER.fetch_add(1, Ordering::SeqCst) + 1;

    // Simulate occasional API failures for demo purposes
    if call_number % 7 == 0 {
        return Err("Network error - server temporarily unavailable".to_string());
    }

    Ok(UserProfile {
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        last_seen: format!("API call #{call_number}"),
        status: if call_number % 3 == 0 {
            "Away"
        } else {
            "Online"
        }
        .to_string(),
    })
}

/// Parameterized SWR provider for user posts with 2 second stale time
/// Uses intelligent cache management: expires after 20s and cleanup runs every 5s
#[provider(stale_time = "2s", cache_expiration = "20s")]
async fn fetch_user_posts(user_id: u32) -> Result<Vec<Post>, String> {
    sleep(Duration::from_millis(800)).await;

    let call_number = API_CALL_COUNTER.fetch_add(1, Ordering::SeqCst) + 1;

    Ok(vec![
        Post {
            id: 1,
            title: format!("Latest post from user {user_id}"),
            content: format!("This is the latest content (fetch #{call_number})"),
            timestamp: format!("2024-{:02}-15", (call_number % 12) + 1),
        },
        Post {
            id: 2,
            title: format!("Previous post from user {user_id}"),
            content: format!("Some older content (fetch #{call_number})"),
            timestamp: format!("2024-{:02}-10", ((call_number + 1) % 12) + 1),
        },
        Post {
            id: 3,
            title: format!("Archive post from user {user_id}"),
            content: format!("Archived content (fetch #{call_number})"),
            timestamp: format!("2024-{:02}-05", ((call_number + 2) % 12) + 1),
        },
    ])
}

/// Data structures for our demo
#[derive(Debug, Clone, PartialEq)]
pub struct UserProfile {
    name: String,
    email: String,
    last_seen: String,
    status: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Post {
    id: u32,
    title: String,
    content: String,
    timestamp: String,
}

/// Main demo component
#[component]
fn SwrDemo() -> Element {
    let mut selected_user_id = use_signal(|| 1u32);

    // SWR providers - now with automatic staleness checking built into the library!
    // No more manual timers needed - the library automatically checks for stale data
    // and triggers revalidation in the background when data becomes stale.
    let user_profile = use_provider(fetch_user_profile(), ());
    let user_posts = use_provider(fetch_user_posts(), (*selected_user_id.read(),));

    // Manual refresh functions
    let refresh_profile = use_invalidate_provider(fetch_user_profile(), ());
    let refresh_posts = use_invalidate_provider(fetch_user_posts(), *selected_user_id.read());

    rsx! {
        div { class: "swr-demo",
            header { class: "demo-header",
                h1 { "🔄 SWR Pattern Demo" }
                p { class: "demo-description",
                    "Stale-While-Revalidate pattern serves cached data instantly while updating in background."
                }
                div { class: "stale-time-info",
                    span { class: "info-badge", "Profile: stale 3s, expires 30s" }
                    span { class: "info-badge", "Posts: stale 2s, expires 20s" }
                    span { class: "info-badge", "🧠 Smart Cache: ✅ Active" }
                }
            }

            div { class: "demo-controls",
                h3 { "Test SWR Behavior:" }
                div { class: "control-group",
                    button {
                        class: "refresh-btn primary",
                        onclick: move |_| refresh_profile(),
                        "🔄 Refresh Profile"
                    }
                    button {
                        class: "refresh-btn secondary",
                        onclick: move |_| refresh_posts(),
                        "🔄 Refresh Posts"
                    }
                }
                div { class: "user-selector",
                    label { "Select User ID: " }
                    input {
                        r#type: "number",
                        min: "1",
                        max: "5",
                        value: "{selected_user_id.read()}",
                        oninput: move |e| {
                            if let Ok(value) = e.value().parse::<u32>() {
                                selected_user_id.set(value);
                            }
                        },
                    }
                }
            }

            div { class: "data-grid",
                div { class: "data-card profile-card",
                    div { class: "card-header",
                        h3 { "👤 User Profile" }
                        span { class: "cache-indicator", "⚡ SWR Active" }
                    }
                    SWRDataDisplay {
                        data: user_profile,
                        render_success: |profile: &UserProfile| rsx! {
                            div { class: "profile-content",
                                div { class: "profile-info",
                                    h4 { "{profile.name}" }
                                    p { class: "email", "📧 {profile.email}" }
                                    p { class: "status",
                                        span { class: if profile.status == "Online" { "status-online" } else { "status-away" },
                                            "● {profile.status}"
                                        }
                                    }
                                }
                                div { class: "profile-meta",
                                    p { class: "last-seen", "🕒 {profile.last_seen}" }
                                }
                            }
                        },
                    }
                }

                div { class: "data-card posts-card",
                    div { class: "card-header",
                        h3 { "📝 User Posts" }
                        span { class: "cache-indicator", "⚡ SWR Active" }
                    }
                    SWRDataDisplay {
                        data: user_posts,
                        render_success: |posts: &Vec<Post>| rsx! {
                            div { class: "posts-content",
                                for post in posts.iter().take(2) {
                                    div { class: "post-item",
                                        h5 { class: "post-title", "{post.title}" }
                                        p { class: "post-content", "{post.content}" }
                                        span { class: "post-timestamp", "📅 {post.timestamp}" }
                                    }
                                }
                                if posts.len() > 2 {
                                    p { class: "more-posts", "... and {posts.len() - 2} more posts" }
                                }
                            }
                        },
                    }
                }
            }

            footer { class: "demo-footer",
                p { class: "instructions",
                    "💡 Try: Refresh data, wait for stale time to pass, then change user ID to see SWR in action!"
                }
            }
        }

        style { {include_str!("./assets/swr_demo_styles.css")} }
    }
}

/// Reusable component for displaying SWR data with consistent loading/error handling
#[component]
fn SWRDataDisplay<
    T: 'static + Clone + PartialEq,
    E: 'static + Clone + PartialEq + std::fmt::Display,
>(
    data: Signal<State<T, E>>,
    render_success: fn(&T) -> Element,
) -> Element {
    match &*data.read() {
        State::Loading { .. } => rsx! {
            div { class: "loading-container",
                div { class: "loading-spinner" }
                span { "Fetching fresh data..." }
            }
        },
        State::Error(e) => rsx! {
            div { class: "error-container",
                span { class: "error-icon", "❌" }
                span { class: "error-message", "Error: {e}" }
            }
        },
        State::Success(value) => render_success(value),
    }
}

/// Application root - now using global providers for simplified setup
fn app() -> Element {
    rsx! {
        SwrDemo {}
    }
}

fn main() {
    // Initialize global providers for application-wide cache management
    let _ = dioxus_provider::init();

    println!("🚀 Starting SWR Demo");
    println!("🔄 Demonstrating Stale-While-Revalidate pattern");

    launch(app);
}
