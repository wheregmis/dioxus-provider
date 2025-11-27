//! Features Overview
//!
//! This example demonstrates the core features of `dioxus-provider` in a single application:
//! 1. Basic Fetching (`use_provider`)
//! 2. Parameterized Fetching
//! 3. Cache Invalidation & Mutations
//! 4. Dependency Injection
//!
//! Run with: dx serve --example features_overview

use dioxus::prelude::*;
use dioxus_provider::prelude::*;
use std::time::Duration;

// ============================================================================
// 1. DATA TYPES
// ============================================================================

#[derive(Clone, Debug, PartialEq)]
struct User {
    id: u32,
    name: String,
    email: String,
}

#[derive(Clone, Debug, PartialEq)]
struct Post {
    id: u32,
    title: String,
}

// ============================================================================
// 2. DEPENDENCIES (for Dependency Injection)
// ============================================================================

#[derive(Clone)]
struct ApiClient {
    base_url: String,
}

impl ApiClient {
    fn new(base_url: String) -> Self {
        Self { base_url }
    }

    async fn fetch_user(&self, id: u32) -> Result<User, String> {
        // Simulate network delay
        #[cfg(not(target_family = "wasm"))]
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        Ok(User {
            id,
            name: format!("User {} from {}", id, self.base_url),
            email: format!("user{}@example.com", id),
        })
    }
}

// ============================================================================
// 3. PROVIDER FUNCTIONS
// ============================================================================

/// Basic provider: Fetch a user by ID
/// Demonstrates: Dependency Injection + Parameters
async fn fetch_user(id: u32) -> Result<User, String> {
    // Inject the API client
    let client = inject::<ApiClient>().map_err(|e| e.to_string())?;
    client.fetch_user(id).await
}

/// Basic provider: Fetch all posts
/// Demonstrates: Simple fetching
async fn fetch_posts() -> Result<Vec<Post>, String> {
    #[cfg(not(target_family = "wasm"))]
    tokio::time::sleep(Duration::from_millis(800)).await;

    Ok(vec![
        Post { id: 1, title: "Hello World".to_string() },
        Post { id: 2, title: "Dioxus Provider is Cool".to_string() },
    ])
}

/// Mutation: Update a user's name
/// Demonstrates: Mutations + Invalidation
async fn update_user_name(id: u32, new_name: String) -> Result<User, String> {
    #[cfg(not(target_family = "wasm"))]
    tokio::time::sleep(Duration::from_millis(500)).await;

    Ok(User {
        id,
        name: new_name,
        email: format!("user{}@example.com", id),
    })
}

// ============================================================================
// 4. COMPONENTS
// ============================================================================

fn main() {
    // 1. Initialize Dependencies
    ensure_dependency(ApiClient::new("https://api.example.com".to_string()))
        .expect("Failed to register dependency");

    // 2. Initialize Provider Runtime
    dioxus_provider::init().expect("Failed to init provider runtime");

    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut active_tab = use_signal(|| "basic");

    rsx! {
        div { style: "font-family: sans-serif; max-width: 800px; margin: 0 auto; padding: 20px;",
            h1 { "Dioxus Provider: Features Overview" }
            
            // Tab Navigation
            div { style: "display: flex; gap: 10px; margin-bottom: 20px; flex-wrap: wrap;",
                button { 
                    onclick: move |_| active_tab.set("basic"),
                    disabled: *active_tab.read() == "basic",
                    "Basic & Lifecycle" 
                }
                button { 
                    onclick: move |_| active_tab.set("params"),
                    disabled: *active_tab.read() == "params",
                    "Parameterized & DI" 
                }
                button { 
                    onclick: move |_| active_tab.set("composition"),
                    disabled: *active_tab.read() == "composition",
                    "Composition" 
                }
                button { 
                    onclick: move |_| active_tab.set("mutation"),
                    disabled: *active_tab.read() == "mutation",
                    "Mutations" 
                }
            }

            // Tab Content
            match *active_tab.read() {
                "basic" => rsx! { BasicDemo {} },
                "params" => rsx! { ParamsDemo {} },
                "composition" => rsx! { CompositionDemo {} },
                "mutation" => rsx! { MutationDemo {} },
                _ => rsx! { "Unknown tab" }
            }
        }
    }
}

/// Demonstrates basic fetching with Lifecycle Controls (SWR)
#[component]
fn BasicDemo() -> Element {
    // Fetch posts with a 5-second stale time (SWR)
    let mut posts = use_provider(fetch_posts)
        .stale_time(Duration::from_secs(5));

    use_effect(move || {
        posts.call();
    });

    rsx! {
        div { style: "border: 1px solid #ccc; padding: 20px; border-radius: 8px;",
            h2 { "Basic Fetching & Lifecycle" }
            p { "Fetches a list of posts. Data is considered fresh for 5 seconds." }
            
            match posts.state() {
                State::Pending => rsx! { "Loading posts..." },
                State::Error => rsx! { "Error: {posts.error().unwrap()}" },
                State::Ready => {
                    if let Some(data) = posts.get_data() {
                        rsx! {
                            ul {
                                for post in data {
                                    li { "{post.title}" }
                                }
                            }
                            div { style: "color: #666; font-size: 0.9em;",
                                "Click Refresh. If < 5s passed, no network request (check console/logs)."
                            }
                        }
                    } else {
                        rsx! { "No data" }
                    }
                }
                _ => rsx! { "Idle" }
            }
            
            button { onclick: move |_| posts.call(), "Refresh" }
        }
    }
}

/// Demonstrates composing multiple providers (Parallel Fetching)
#[component]
fn CompositionDemo() -> Element {
    let mut user = use_provider(fetch_user);
    let mut posts = use_provider(fetch_posts);

    use_effect(move || {
        // Fetch both in parallel
        user.call(1);
        posts.call();
    });

    rsx! {
        div { style: "border: 1px solid #ccc; padding: 20px; border-radius: 8px;",
            h2 { "Provider Composition" }
            p { "Fetching User and Posts in parallel." }

            div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 20px;",
                div {
                    h3 { "User" }
                    match user.state() {
                        State::Pending => rsx! { "Loading user..." },
                        State::Ready => {
                            if let Some(u) = user.get_data() {
                                rsx! { "{u.name}" }
                            } else { rsx! { "No user" } }
                        },
                        _ => rsx! { "..." }
                    }
                }
                div {
                    h3 { "Posts" }
                    match posts.state() {
                        State::Pending => rsx! { "Loading posts..." },
                        State::Ready => {
                            if let Some(p) = posts.get_data() {
                                rsx! { "Loaded {p.len()} posts" }
                            } else { rsx! { "No posts" } }
                        },
                        _ => rsx! { "..." }
                    }
                }
            }
        }
    }
}

/// Demonstrates parameterized fetching and dependency injection
#[component]
fn ParamsDemo() -> Element {
    let mut user_id = use_signal(|| 1u32);
    let mut user = use_provider(fetch_user);

    use_effect(move || {
        user.call(*user_id.read());
    });

    rsx! {
        div { style: "border: 1px solid #ccc; padding: 20px; border-radius: 8px;",
            h2 { "Parameterized & DI" }
            p { "Fetches a user by ID. The provider uses an injected `ApiClient`." }
            
            div { style: "margin-bottom: 10px;",
                "User ID: "
                input { 
                    r#type: "number", 
                    value: "{user_id}",
                    oninput: move |e| if let Ok(n) = e.value().parse() { user_id.set(n) }
                }
            }

            match user.state() {
                State::Pending => rsx! { "Loading user..." },
                State::Error => rsx! { "Error: {user.error().unwrap()}" },
                State::Ready => {
                    if let Some(u) = user.get_data() {
                        rsx! {
                            div {
                                b { "{u.name}" }
                                br {}
                                "{u.email}"
                            }
                        }
                    } else {
                        rsx! { "No data" }
                    }
                }
                _ => rsx! { "Idle" }
            }
        }
    }
}

/// Demonstrates mutations and cache invalidation
#[component]
fn MutationDemo() -> Element {
    let mut user = use_provider(fetch_user);
    // Mutation that invalidates the specific user cache entry
    let mut update = use_mutation(update_user_name)
        .invalidate_all(fetch_user); // Invalidate all user cache entries for simplicity

    // Fetch user 1 initially
    use_effect(move || {
        user.call(1);
    });

    rsx! {
        div { style: "border: 1px solid #ccc; padding: 20px; border-radius: 8px;",
            h2 { "Mutations & Invalidation" }
            p { "Updates User 1's name and automatically refreshes the data." }

            div { style: "margin-bottom: 20px;",
                "Current User 1 Data: "
                if let Some(u) = user.get_data() {
                    b { "{u.name}" }
                } else {
                    "Loading..."
                }
            }

            button {
                disabled: update.pending(),
                onclick: move |_| {
                    update.call(1, "Alice (Updated)".to_string());
                },
                if update.pending() { "Updating..." } else { "Update Name to 'Alice (Updated)'" }
            }
            
            button {
                disabled: update.pending(),
                onclick: move |_| {
                    update.call(1, "Bob (Updated)".to_string());
                },
                if update.pending() { "Updating..." } else { "Update Name to 'Bob (Updated)'" }
            }
        }
    }
}
