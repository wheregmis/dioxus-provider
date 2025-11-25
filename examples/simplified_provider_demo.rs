//! Simplified Provider Demo with Stores
//!
//! This example demonstrates the new simplified provider API that works with
//! any async function, using Dioxus Stores for fine-grained reactivity.
//!
//! Run with: dx serve --example simplified_provider_demo

use dioxus::prelude::*;
use std::time::Duration;

// Import the new simplified API
use dioxus_provider::provider::{use_provider, ProviderState};
use dioxus_provider::mutation_new::{use_mutation, MutationState};
use dioxus_provider::global::init;

const STYLE: &str = r#"
    body {
        font-family: 'Segoe UI', system-ui, sans-serif;
        background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%);
        color: #eee;
        min-height: 100vh;
        margin: 0;
        padding: 20px;
    }
    .container {
        max-width: 800px;
        margin: 0 auto;
    }
    h1 {
        color: #00d9ff;
        text-align: center;
        margin-bottom: 30px;
    }
    .card {
        background: rgba(255, 255, 255, 0.05);
        border-radius: 12px;
        padding: 20px;
        margin-bottom: 20px;
        border: 1px solid rgba(255, 255, 255, 0.1);
    }
    .card h2 {
        color: #ff6b9d;
        margin-top: 0;
    }
    button {
        background: linear-gradient(135deg, #00d9ff, #00ff88);
        border: none;
        padding: 10px 20px;
        border-radius: 8px;
        color: #1a1a2e;
        font-weight: bold;
        cursor: pointer;
        margin-right: 10px;
        margin-bottom: 10px;
        transition: transform 0.2s;
    }
    button:hover {
        transform: scale(1.05);
    }
    button:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }
    .status {
        padding: 8px 16px;
        border-radius: 20px;
        display: inline-block;
        margin: 10px 0;
    }
    .status.pending {
        background: #ffd93d;
        color: #1a1a2e;
    }
    .status.ready {
        background: #00ff88;
        color: #1a1a2e;
    }
    .status.error {
        background: #ff6b6b;
        color: white;
    }
    .status.idle {
        background: #666;
        color: white;
    }
    .data {
        background: rgba(0, 217, 255, 0.1);
        padding: 15px;
        border-radius: 8px;
        margin-top: 10px;
        font-family: monospace;
    }
    .user-card {
        display: flex;
        align-items: center;
        gap: 15px;
        padding: 15px;
        background: rgba(255, 107, 157, 0.1);
        border-radius: 8px;
        margin-top: 10px;
    }
    .user-avatar {
        width: 60px;
        height: 60px;
        border-radius: 50%;
        background: linear-gradient(135deg, #ff6b9d, #00d9ff);
        display: flex;
        align-items: center;
        justify-content: center;
        font-size: 24px;
    }
    input {
        padding: 8px 12px;
        border-radius: 6px;
        border: 1px solid #444;
        background: #2a2a4a;
        color: #eee;
        margin-right: 10px;
    }
"#;

fn main() {
    // Initialize the global provider runtime
    init().expect("Failed to initialize provider runtime");
    dioxus::launch(app);
}

// ============================================================================
// ASYNC FUNCTIONS - These are plain async functions, no macros needed!
// ============================================================================

/// Fetch a user by ID - simulates an API call
async fn fetch_user(id: u32) -> Result<User, String> {
    // Simulate network delay
    #[cfg(not(target_family = "wasm"))]
    tokio::time::sleep(Duration::from_millis(800)).await;
    #[cfg(target_family = "wasm")]
    wasmtimer::tokio::sleep(Duration::from_millis(800)).await;

    if id == 0 {
        return Err("User ID cannot be 0".to_string());
    }

    Ok(User {
        id,
        name: format!("User {}", id),
        email: format!("user{}@example.com", id),
    })
}

/// Fetch all users - simulates an API call
async fn fetch_users() -> Result<Vec<User>, String> {
    #[cfg(not(target_family = "wasm"))]
    tokio::time::sleep(Duration::from_millis(1000)).await;
    #[cfg(target_family = "wasm")]
    wasmtimer::tokio::sleep(Duration::from_millis(1000)).await;

    Ok(vec![
        User { id: 1, name: "Alice".to_string(), email: "alice@example.com".to_string() },
        User { id: 2, name: "Bob".to_string(), email: "bob@example.com".to_string() },
        User { id: 3, name: "Charlie".to_string(), email: "charlie@example.com".to_string() },
    ])
}

/// Update a user - simulates an API mutation
async fn update_user(id: u32, name: String) -> Result<User, String> {
    #[cfg(not(target_family = "wasm"))]
    tokio::time::sleep(Duration::from_millis(500)).await;
    #[cfg(target_family = "wasm")]
    wasmtimer::tokio::sleep(Duration::from_millis(500)).await;

    Ok(User {
        id,
        name,
        email: format!("user{}@example.com", id),
    })
}

#[derive(Clone, Debug, PartialEq)]
struct User {
    id: u32,
    name: String,
    email: String,
}

// ============================================================================
// COMPONENTS
// ============================================================================

fn app() -> Element {
    rsx! {
        style { {STYLE} }
        div { class: "container",
            h1 { "🚀 Simplified Provider Demo (with Stores)" }
            p { style: "text-align: center; color: #888;",
                "Using Dioxus Stores for fine-grained reactivity"
            }
            
            SingleUserDemo {}
            UserListDemo {}
            MutationDemo {}
        }
    }
}

/// Demo: Fetching a single user with configurable caching
#[component]
fn SingleUserDemo() -> Element {
    let mut user_id = use_signal(|| 1u32);
    
    // Create a provider for fetching users
    // Works with any async fn that returns Result<T, E>!
    let mut user_provider = use_provider(fetch_user)
        .stale_time(Duration::from_secs(30))
        .cache_expiration(Duration::from_secs(120));

    // Fetch on mount and when user_id changes
    use_effect(move || {
        user_provider.call(*user_id.read());
    });

    // Access state with fine-grained reactivity via Store
    let state = user_provider.state();
    
    let status_class = match state {
        ProviderState::Idle => "idle",
        ProviderState::Pending => "pending",
        ProviderState::Ready => "ready",
        ProviderState::Errored => "error",
        ProviderState::Reset => "idle",
    };

    rsx! {
        div { class: "card",
            h2 { "👤 Single User Fetch" }
            
            div {
                label { "User ID: " }
                input {
                    r#type: "number",
                    value: "{user_id}",
                    oninput: move |e| {
                        if let Ok(id) = e.value().parse::<u32>() {
                            user_id.set(id);
                        }
                    }
                }
                button {
                    onclick: move |_| user_provider.call(*user_id.read()),
                    disabled: user_provider.pending(),
                    "Fetch User"
                }
                button {
                    onclick: move |_| user_provider.invalidate(),
                    "Invalidate Cache"
                }
            }
            
            div { class: "status {status_class}",
                match state {
                    ProviderState::Idle => "Idle",
                    ProviderState::Pending => "Loading...",
                    ProviderState::Ready => "Ready",
                    ProviderState::Errored => "Error",
                    ProviderState::Reset => "Reset",
                }
            }
            
            // Fine-grained access: only subscribes to value changes
            if let Some(user) = user_provider.get_data() {
                div { class: "user-card",
                    div { class: "user-avatar", "{user.name.chars().next().unwrap_or('?')}" }
                    div {
                        div { strong { "{user.name}" } }
                        div { "{user.email}" }
                        div { "ID: {user.id}" }
                    }
                }
            }
            
            if let Some(error) = user_provider.error() {
                div { class: "data", style: "color: #ff6b6b;",
                    "Error: {error}"
                }
            }
        }
    }
}

/// Demo: Fetching a list of users
#[component]
fn UserListDemo() -> Element {
    let mut users_provider = use_provider(fetch_users)
        .stale_time(Duration::from_secs(60));

    // Fetch on mount
    use_effect(move || {
        users_provider.call();
    });

    rsx! {
        div { class: "card",
            h2 { "👥 User List" }
            
            button {
                onclick: move |_| users_provider.call(),
                disabled: users_provider.pending(),
                "Refresh List"
            }
            
            if users_provider.pending() {
                div { class: "status pending", "Loading users..." }
            }
            
            if let Some(users) = users_provider.get_data() {
                div { class: "data",
                    for user in users.iter() {
                        div { key: "{user.id}",
                            "• {user.name} ({user.email})"
                        }
                    }
                }
            }
        }
    }
}

/// Demo: Mutations with cache invalidation
#[component]
fn MutationDemo() -> Element {
    let mut name = use_signal(|| "Updated Name".to_string());
    
    // Create a mutation that invalidates the user list after success
    let mut update_mutation = use_mutation(update_user)
        .invalidates(fetch_users);

    let state = update_mutation.state();
    
    let status_class = match state {
        MutationState::Idle => "idle",
        MutationState::Pending => "pending",
        MutationState::Success => "ready",
        MutationState::Errored => "error",
        MutationState::Reset => "idle",
    };

    rsx! {
        div { class: "card",
            h2 { "✏️ Mutation Demo" }
            
            div {
                input {
                    r#type: "text",
                    value: "{name}",
                    oninput: move |e| name.set(e.value()),
                    placeholder: "New name"
                }
                button {
                    onclick: move |_| {
                        update_mutation.call(1, name.read().clone());
                    },
                    disabled: update_mutation.pending(),
                    "Update User 1"
                }
                button {
                    onclick: move |_| update_mutation.reset(),
                    "Reset"
                }
            }
            
            div { class: "status {status_class}",
                match state {
                    MutationState::Idle => "Ready to mutate",
                    MutationState::Pending => "Updating...",
                    MutationState::Success => "Success!",
                    MutationState::Errored => "Error",
                    MutationState::Reset => "Reset",
                }
            }
            
            if let Some(Ok(user)) = update_mutation.value() {
                div { class: "data",
                    "Updated: {user.name} (ID: {user.id})"
                }
            }
        }
    }
}
