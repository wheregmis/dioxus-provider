//! # Interval Refresh Demo
//!
//! This example demonstrates automatic interval-based data refresh in dioxus-provider.
//! Providers can be configured to automatically refresh their data at specified intervals,
//! keeping the UI updated with fresh data without manual intervention.
//!
//! ## Key Features Demonstrated:
//! - Automatic background refresh at configurable intervals
//! - Different refresh rates for different data types
//! - Real-time data updates without user interaction
//! - Global provider management
//! - Cache integration with interval refresh

use dioxus::prelude::*;
use dioxus_provider::hooks::ProviderState;
use dioxus_provider::prelude::*;
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

/// Fast refresh provider - updates every 2 seconds
#[provider(interval = "2s", cache_expiration = "10s")]
async fn fetch_live_stats() -> Result<LiveStats, String> {
    println!("🔄 [FETCH] Live stats fetch started");
    sleep(Duration::from_millis(300)).await;

    let call_number = API_CALL_COUNTER.fetch_add(1, Ordering::SeqCst) + 1;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    Ok(LiveStats {
        active_users: (call_number * 13) % 1000 + 100,
        cpu_usage: (call_number * 7) % 100,
        memory_usage: (call_number * 11) % 80 + 20,
        last_updated: timestamp,
        fetch_count: call_number,
    })
}

/// Medium refresh provider - updates every 5 seconds
#[provider(interval = "5s", cache_expiration = "20s")]
async fn fetch_system_metrics() -> Result<SystemMetrics, String> {
    println!("🔄 [FETCH] System metrics fetch started");
    sleep(Duration::from_millis(800)).await;

    let call_number = API_CALL_COUNTER.fetch_add(1, Ordering::SeqCst) + 1;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    Ok(SystemMetrics {
        disk_usage: (call_number * 3) % 90 + 10,
        network_io: format!("{:.1} MB/s", (call_number as f64 * 1.7) % 50.0),
        uptime_hours: call_number * 2,
        last_updated: timestamp,
        fetch_count: call_number,
    })
}

/// Slow refresh provider - updates every 10 seconds
#[provider(interval = "10s", cache_expiration = "30s")]
async fn fetch_business_metrics() -> Result<BusinessMetrics, String> {
    println!("🔄 [FETCH] Business metrics fetch started");
    sleep(Duration::from_millis(1200)).await;

    let call_number = API_CALL_COUNTER.fetch_add(1, Ordering::SeqCst) + 1;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    Ok(BusinessMetrics {
        daily_revenue: (call_number * 127) % 10000 + 5000,
        active_subscriptions: (call_number * 89) % 500 + 200,
        conversion_rate: format!("{:.2}%", (call_number as f64 * 0.3) % 15.0 + 2.0),
        last_updated: timestamp,
        fetch_count: call_number,
    })
}

#[derive(Clone, PartialEq, Debug)]
pub struct LiveStats {
    active_users: u32,
    cpu_usage: u32,
    memory_usage: u32,
    last_updated: u64,
    fetch_count: u32,
}

#[derive(Clone, PartialEq, Debug)]
pub struct SystemMetrics {
    disk_usage: u32,
    network_io: String,
    uptime_hours: u32,
    last_updated: u64,
    fetch_count: u32,
}

#[derive(Clone, PartialEq, Debug)]
pub struct BusinessMetrics {
    daily_revenue: u32,
    active_subscriptions: u32,
    conversion_rate: String,
    last_updated: u64,
    fetch_count: u32,
}

fn main() {
    // Initialize global providers for application-wide cache management
    dioxus_provider::init();

    println!("🚀 Starting Interval Refresh Demo");
    println!("⏰ Demonstrating automatic background data refresh at configurable intervals");

    launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        div { class: "container",
            style { {include_str!("./assets/style.css")} }

            div { class: "header",
                h1 { class: "main-title", "⏱️ Interval Refresh Demo" }
                p { class: "subtitle", "Automatic Background Data Updates" }
            }

            div { class: "dashboard",
                LiveStatsCard { }
                SystemMetricsCard { }
                BusinessMetricsCard { }
            }

            div { class: "info",
                h3 { "⚡ Automatic Updates" }
                p { "All data refreshes automatically in the background without any user interaction!" }
                ul {
                    li { "🟢 Live Stats: Updates every 2 seconds" }
                    li { "🔵 System Metrics: Updates every 5 seconds" }
                    li { "🟣 Business Metrics: Updates every 10 seconds" }
                }
                p { class: "note",
                    "💡 Watch the fetch count and timestamps to see automatic updates in action!"
                }
            }
        }
    }
}

#[component]
fn LiveStatsCard() -> Element {
    let data = use_provider(fetch_live_stats(), ());

    rsx! {
        MetricsCard {
            title: "Live Stats (2s interval)".to_string(),
            description: "High-frequency metrics that update every 2 seconds".to_string(),
            data: data,
            color_class: "live".to_string(),
        }
    }
}

#[component]
fn SystemMetricsCard() -> Element {
    let data = use_provider(fetch_system_metrics(), ());

    rsx! {
        div { class: "metrics-card system",
            div { class: "card-header",
                h3 { "System Metrics (5s interval)" }
                div { class: match &*data.read() {
                    ProviderState::Loading { .. } => "status loading",
                    ProviderState::Error(_) => "status error",
                    ProviderState::Success(_) => "status success",
                }}
            }

            p { class: "description", "Medium-frequency system monitoring every 5 seconds" }

            div { class: "card-content",
                match &*data.read() {
                    ProviderState::Loading { .. } => rsx! {
                        div { class: "loading-state",
                            div { class: "spinner" }
                            span { "Loading..." }
                        }
                    },
                    ProviderState::Error(e) => rsx! {
                        div { class: "error-state",
                            span { "❌ Error: {e}" }
                        }
                    },
                    ProviderState::Success(metrics) => rsx! {
                        div { class: "data-grid",
                            div { class: "metric",
                                span { class: "label", "Disk Usage:" }
                                span { class: "value", "{metrics.disk_usage}%" }
                            }
                            div { class: "metric",
                                span { class: "label", "Network I/O:" }
                                span { class: "value", "{metrics.network_io}" }
                            }
                            div { class: "metric",
                                span { class: "label", "Uptime:" }
                                span { class: "value", "{metrics.uptime_hours}h" }
                            }
                            div { class: "metric",
                                span { class: "label", "Fetch Count:" }
                                span { class: "value fetch-count", "#{metrics.fetch_count}" }
                            }
                            div { class: "metric timestamp",
                                span { class: "label", "Last Updated:" }
                                span { class: "value", "{metrics.last_updated}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn BusinessMetricsCard() -> Element {
    let data = use_provider(fetch_business_metrics(), ());

    rsx! {
        div { class: "metrics-card business",
            div { class: "card-header",
                h3 { "Business Metrics (10s interval)" }
                div { class: match &*data.read() {
                    ProviderState::Loading { .. } => "status loading",
                    ProviderState::Error(_) => "status error",
                    ProviderState::Success(_) => "status success",
                }}
            }

            p { class: "description", "Low-frequency business data every 10 seconds" }

            div { class: "card-content",
                match &*data.read() {
                    ProviderState::Loading { .. } => rsx! {
                        div { class: "loading-state",
                            div { class: "spinner" }
                            span { "Loading..." }
                        }
                    },
                    ProviderState::Error(e) => rsx! {
                        div { class: "error-state",
                            span { "❌ Error: {e}" }
                        }
                    },
                    ProviderState::Success(metrics) => rsx! {
                        div { class: "data-grid",
                            div { class: "metric",
                                span { class: "label", "Daily Revenue:" }
                                span { class: "value", "${metrics.daily_revenue}" }
                            }
                            div { class: "metric",
                                span { class: "label", "Subscriptions:" }
                                span { class: "value", "{metrics.active_subscriptions}" }
                            }
                            div { class: "metric",
                                span { class: "label", "Conversion:" }
                                span { class: "value", "{metrics.conversion_rate}" }
                            }
                            div { class: "metric",
                                span { class: "label", "Fetch Count:" }
                                span { class: "value fetch-count", "#{metrics.fetch_count}" }
                            }
                            div { class: "metric timestamp",
                                span { class: "label", "Last Updated:" }
                                span { class: "value", "{metrics.last_updated}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn MetricsCard(
    title: String,
    description: String,
    data: Signal<ProviderState<LiveStats, String>>,
    color_class: String,
) -> Element {
    let status_class = match &*data.read() {
        ProviderState::Loading { .. } => "status loading",
        ProviderState::Error(_) => "status error",
        ProviderState::Success(_) => "status success",
    };

    rsx! {
        div { class: "metrics-card {color_class}",
            div { class: "card-header",
                h3 { "{title}" }
                div { class: "{status_class}" }
            }

            p { class: "description", "{description}" }

            div { class: "card-content",
                match &*data.read() {
                    ProviderState::Loading { .. } => rsx! {
                        div { class: "loading-state",
                            div { class: "spinner" }
                            span { "Loading..." }
                        }
                    },
                    ProviderState::Error(e) => rsx! {
                        div { class: "error-state",
                            span { "❌ Error: {e}" }
                        }
                    },
                    ProviderState::Success(stats) => rsx! {
                        div { class: "data-grid",
                            div { class: "metric",
                                span { class: "label", "Active Users:" }
                                span { class: "value", "{stats.active_users}" }
                            }
                            div { class: "metric",
                                span { class: "label", "CPU Usage:" }
                                span { class: "value", "{stats.cpu_usage}%" }
                            }
                            div { class: "metric",
                                span { class: "label", "Memory:" }
                                span { class: "value", "{stats.memory_usage}%" }
                            }
                            div { class: "metric",
                                span { class: "label", "Fetch Count:" }
                                span { class: "value fetch-count", "#{stats.fetch_count}" }
                            }
                            div { class: "metric timestamp",
                                span { class: "label", "Last Updated:" }
                                span { class: "value", "{stats.last_updated}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
