use dioxus::prelude::*;
use dioxus_provider::{hooks::SuspenseSignalExt, prelude::*};
use std::time::Duration;

// A simple provider that simulates a delayed async fetch
#[provider(stale_time = "1s", cache_expiration = "10s")]
async fn fetch_user(id: u32) -> Result<String, String> {
    // Simulate network delay
    tokio::time::sleep(Duration::from_millis(1200)).await;
    if id == 0 {
        Err("User not found".to_string())
    } else {
        Ok(format!("User #{id}"))
    }
}

#[component]
fn UserCard(id: u32) -> Element {
    // Use the provider and suspend rendering until data is ready
    let user = use_provider(fetch_user(), id).suspend()?;

    match user {
        Ok(name) => rsx!(div { "Loaded: {name}" }),
        Err(err) => rsx!(div { "Error: {err}" }),
    }
}

#[component]
fn App() -> Element {
    let mut user_id = use_signal(|| 1u32);

    rsx! {
        div { class: "container",
            h1 { "Suspense Demo with dioxus-provider" }
            button {
                onclick: move |_| user_id += 1,
                "Next User"
            }
            SuspenseBoundary {
                fallback: |_| rsx!(div { "Loading user..." }),
                UserCard { id: *user_id.read() }
            }
        }
    }
}

fn main() {
    let _ = dioxus_provider::init();
    launch(App);
}
