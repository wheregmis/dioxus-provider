//! Cache Expiration Test
//!
//! This example tests cache expiration behavior to ensure that when cache
//! expires, components immediately show loading state and then fetch fresh data.

use dioxus::prelude::*;
use dioxus_provider::hooks::ProviderState;
use dioxus_provider::{global::init_global_providers, prelude::*};
use std::time::Duration;

// Cross-platform sleep function
#[cfg(not(target_family = "wasm"))]
use tokio::time::sleep;
#[cfg(target_family = "wasm")]
use wasmtimer::tokio::sleep;

/// Test provider with very short cache expiration (5 seconds)
#[provider(cache_expiration = "5s")]
async fn fetch_test_data() -> Result<String, String> {
    // Add some delay to simulate network request
    sleep(Duration::from_millis(1000)).await;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    println!("🔄 [PROVIDER] Executing provider at timestamp: {timestamp}");
    Ok(format!("Data fetched at: {timestamp}"))
}

#[component]
fn App() -> Element {
    // Get the test data
    let data = use_provider(fetch_test_data(), ());

    // Manual refresh function
    let refresh = use_invalidate_provider(fetch_test_data(), ());

    println!("🖥️ [UI] Rendering component");

    rsx! {
        div { style: "padding: 20px;",
            h1 { "Reactive Cache Expiration Test" }
            p { "This test has a 5-second cache expiration with automatic reactive monitoring" }
            p { "No forced re-renders needed - cache expiration triggers reactive updates" }

            div { style: "margin: 20px 0; padding: 20px; border: 1px solid #ccc;",
                h3 { "Test Data (expires in 5s):" }
                match &*data.read() {
                    ProviderState::Loading { .. } => rsx! {
                        div { style: "color: orange;",
                            "🔄 Loading data..."
                        }
                    },
                    ProviderState::Success(result) => rsx! {
                        div { style: "color: green;",
                            "✅ Success: {result}"
                        }
                    },
                    ProviderState::Error(err) => rsx! {
                        div { style: "color: red;",
                            "❌ Error: {err}"
                        }
                    },
                }

                button {
                    onclick: move |_| {
                        println!("🔄 [MANUAL] Manual refresh triggered");
                        refresh();
                    },
                    style: "margin-top: 10px; padding: 10px;",
                    "🔄 Manual Refresh"
                }
            }
        }
    }
}

fn main() {
    // Initialize global providers
    dioxus_provider::init();

    println!("🚀 Starting Reactive Cache Expiration Test");
    println!("📋 Expected behavior:");
    println!("   1. Data loads initially");
    println!("   2. Cache expiration monitoring starts automatically");
    println!("   3. After 5 seconds, cache expiration task triggers refresh");
    println!("   4. Component shows Loading, then fetches new data reactively");

    dioxus::launch(App);
}
