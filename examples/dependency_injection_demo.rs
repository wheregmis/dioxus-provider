/*!
 * Dependency Injection Demo
 *
 * Demonstrates how to use the macro-based dependency injection system
 * to manage non-parameter dependencies like API clients and databases.
 */
#![allow(dead_code)]

use dioxus::prelude::*;
use dioxus_provider::prelude::*;

// Example dependencies that don't implement PartialEq/Hash
#[derive(Clone)]
struct ApiClient {
    base_url: String,
    auth_token: String,
}

impl ApiClient {
    fn new(base_url: String, auth_token: String) -> Self {
        Self {
            base_url,
            auth_token,
        }
    }

    async fn fetch_user(&self, id: u32) -> Result<User, String> {
        // Simulate API call
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        Ok(User {
            id,
            name: format!("User {} from {}", id, self.base_url),
            email: format!("user{id}@example.com"),
        })
    }

    async fn fetch_posts(&self, user_id: u32) -> Result<Vec<Post>, String> {
        // Simulate API call
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        Ok(vec![
            Post {
                id: 1,
                user_id,
                title: "First Post".to_string(),
                content: "Content from API".to_string(),
            },
            Post {
                id: 2,
                user_id,
                title: "Second Post".to_string(),
                content: "More content".to_string(),
            },
        ])
    }
}

#[derive(Clone)]
struct Database {
    connection_string: String,
}

impl Database {
    fn new(connection_string: String) -> Self {
        Self { connection_string }
    }

    async fn log_access(&self, user_id: u32, resource: &str) -> Result<(), String> {
        // Simulate database write
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        println!("DB LOG: User {user_id} accessed {resource}");
        Ok(())
    }
}

// Data structures
#[derive(Clone, Debug, PartialEq)]
pub struct User {
    id: u32,
    name: String,
    email: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Post {
    id: u32,
    user_id: u32,
    title: String,
    content: String,
}

// Macro-based dependency injection providers (clean and simple!)

#[provider]
async fn fetch_user(user_id: u32) -> Result<User, String> {
    println!("fetch_user called with user_id: {user_id}");

    // Dependencies are automatically injected via the inject() function
    let api_client = inject::<ApiClient>()?;
    let database = inject::<Database>()?;

    // Log access
    database.log_access(user_id, "user_profile").await?;

    // Fetch user
    api_client.fetch_user(user_id).await
}

#[provider]
async fn fetch_user_posts(user_id: u32) -> Result<Vec<Post>, String> {
    println!("fetch_user_posts called with user_id: {user_id}");

    let api_client = inject::<ApiClient>()?;
    let database = inject::<Database>()?;

    // Log access
    database.log_access(user_id, "user_posts").await?;

    // Fetch posts
    api_client.fetch_posts(user_id).await
}

#[provider(cache_expiration = "30s")]
async fn fetch_user_with_cache(user_id: u32) -> Result<User, String> {
    println!("fetch_user_with_cache called with user_id: {user_id}");

    let api_client = inject::<ApiClient>()?;
    let database = inject::<Database>()?;

    // Log access
    database.log_access(user_id, "cached_user_profile").await?;

    // Fetch user with 30 second cache
    api_client.fetch_user(user_id).await
}

#[provider(stale_time = "10s")]
async fn fetch_fresh_posts(user_id: u32) -> Result<Vec<Post>, String> {
    println!("fetch_fresh_posts called with user_id: {user_id}");

    let api_client = inject::<ApiClient>()?;
    let database = inject::<Database>()?;

    // Log access
    database.log_access(user_id, "fresh_posts").await?;

    // Fetch posts with stale-while-revalidate
    api_client.fetch_posts(user_id).await
}

// Component using dependency-injected providers
#[component]
fn UserProfile(user_id: u32) -> Element {
    // Add debug output
    println!("UserProfile rendering for user_id: {user_id}");

    // Use macro-generated providers that depend on injected dependencies
    let user = use_provider(fetch_user(), user_id);
    let posts = use_provider(fetch_user_posts(), user_id);

    rsx! {
        div {
            class: "user-profile",
            h3 { "User Profile (Real-time)" }
            p {
                style: "font-weight: bold; color: #007acc;",
                "User ID: {user_id}"
            }

            match user() {
                State::Loading { .. } => rsx! {
                    div { class: "loading", "Loading user..." }
                },
                State::Success(user) => rsx! {
                    div { class: "user-info",
                        h2 { "{user.name}" }
                        p { "Email: {user.email}" }
                        p { "ID: {user.id}" }
                    }
                },
                State::Error(err) => rsx! {
                    div { class: "error", "Error loading user: {err}" }
                },
            }

            match posts() {
                State::Loading { .. } => rsx! {
                    div { class: "loading", "Loading posts..." }
                },
                State::Success(posts) => rsx! {
                    div { class: "posts",
                        h3 { "Posts" }
                        for post in posts {
                            div { class: "post",
                                h4 { "{post.title}" }
                                p { "{post.content}" }
                            }
                        }
                    }
                },
                State::Error(err) => rsx! {
                    div { class: "error", "Error loading posts: {err}" }
                },
            }
        }
    }
}

#[component]
fn CachedUserProfile(user_id: u32) -> Element {
    // Add debug output
    println!("CachedUserProfile rendering for user_id: {user_id}");

    // Use cached provider that demonstrates different cache strategies
    let cached_user = use_provider(fetch_user_with_cache(), user_id);
    let fresh_posts = use_provider(fetch_fresh_posts(), user_id);

    rsx! {
        div {
            class: "cached-user-profile",
            style: "border: 2px solid #007acc; padding: 15px; margin: 15px 0; border-radius: 8px;",

            h3 { "Cached User Profile (30s cache)" }
            p { style: "color: #666; font-size: 0.9em;", "User ID: {user_id}" }

            match cached_user() {
                State::Loading { .. } => rsx! {
                    div { class: "loading", "Loading cached user..." }
                },
                State::Success(user) => rsx! {
                    div { class: "user-info",
                        h4 { "{user.name}" }
                        p { "Email: {user.email}" }
                        small { "This data is cached for 30 seconds" }
                    }
                },
                State::Error(err) => rsx! {
                    div { class: "error", "Error loading cached user: {err}" }
                },
            }

            h4 { "Fresh Posts (10s stale time)" }
            match fresh_posts() {
                State::Loading { .. } => rsx! {
                    div { class: "loading", "Loading fresh posts..." }
                },
                State::Success(posts) => rsx! {
                    div { class: "posts",
                        for post in posts {
                            div { class: "post",
                                style: "border-left: 3px solid #007acc; padding-left: 10px; margin: 5px 0;",
                                h5 { "{post.title}" }
                                p {
                                    style: "font-size: 0.9em; color: #666;",
                                    "{post.content}"
                                }
                            }
                        }
                        small { "Posts become stale after 10 seconds but are refreshed in background" }
                    }
                },
                State::Error(err) => rsx! {
                    div { class: "error", "Error loading fresh posts: {err}" }
                },
            }
        }
    }
}

#[component]
fn App() -> Element {
    let mut selected_user = use_signal(|| 1u32);

    rsx! {
        div {
            class: "app",
            style: "font-family: Arial, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px;",

            h1 { "Dependency Injection Demo" }
            p {
                style: "color: #666; margin-bottom: 20px;",
                "This demo shows how to use macro-based dependency injection with API clients and databases that don't implement PartialEq/Hash."
            }

            div { class: "controls",
                style: "margin-bottom: 20px; padding: 15px; background: #f5f5f5; border-radius: 8px;",
                label {
                    style: "font-weight: bold; margin-right: 10px;",
                    "Select User: "
                }
                select {
                    value: "{selected_user}",
                    style: "padding: 5px 10px; border: 1px solid #ccc; border-radius: 4px;",
                    onchange: move |evt| {
                        if let Ok(user_id) = evt.value().parse::<u32>() {
                            selected_user.set(user_id);
                        }
                    },
                    option { value: "1", "User 1" }
                    option { value: "2", "User 2" }
                    option { value: "3", "User 3" }
                }
            }

            div { class: "content",
                UserProfile { user_id: selected_user() }
                CachedUserProfile { user_id: selected_user() }
            }

            div { class: "info",
                style: "margin-top: 30px; padding: 15px; background: #e8f4f8; border-left: 4px solid #007acc; border-radius: 4px;",
                h3 { "How it works:" }
                ul {
                    li { "API clients and databases are registered globally at startup" }
                    li { "Providers use inject::<T>() to access these dependencies" }
                    li { "The #[provider] macro generates clean, functional providers" }
                    li { "Different cache strategies can be applied with macro arguments" }
                    li { "Dependencies don't need to implement PartialEq or Hash" }
                }
            }
        }
    }
}

/// Initialize all dependencies
fn init_dependencies() -> Result<(), String> {
    // Note: Dependency injection is initialized via dioxus_provider::init() in main()

    // Register API client if not already registered
    if !has_dependency::<ApiClient>() {
        let api_client = ApiClient::new(
            "https://api.example.com".to_string(),
            "secret-token".to_string(),
        );
        register_dependency(api_client)?;
    }

    // Register database if not already registered
    if !has_dependency::<Database>() {
        let database = Database::new("postgresql://localhost/myapp".to_string());
        register_dependency(database)?;
    }

    Ok(())
}

fn main() {
    // Initialize dependencies first
    if let Err(e) = init_dependencies() {
        eprintln!("Failed to initialize dependencies: {e}");
        std::process::exit(1);
    }

    // Initialize global providers
    if let Err(err) = dioxus_provider::init() {
        eprintln!("❌ Failed to initialize dioxus_provider: {err}");
        std::process::exit(1);
    }

    // Launch the app
    dioxus::launch(App);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dependency_injection() {
        // Initialize dependencies
        init_dependencies().unwrap();

        // Test macro-generated provider
        let provider = fetch_user;
        let user = provider.run(1).await.unwrap();

        assert_eq!(user.id, 1);
        assert!(user.name.contains("User 1"));
        assert!(user.email.contains("user1@"));
    }

    #[tokio::test]
    async fn test_posts_provider() {
        // Initialize dependencies
        init_dependencies().unwrap();

        // Test macro-generated posts provider
        let provider = fetch_user_posts;
        let posts = provider.run(1).await.unwrap();

        assert_eq!(posts.len(), 2);
        assert_eq!(posts[0].user_id, 1);
        assert_eq!(posts[1].user_id, 1);
    }

    #[tokio::test]
    async fn test_cached_provider() {
        // Initialize dependencies
        init_dependencies().unwrap();

        // Test cached provider
        let provider = fetch_user_with_cache;
        let user = provider.run(2).await.unwrap();

        assert_eq!(user.id, 2);
        assert!(user.name.contains("User 2"));
    }
}
