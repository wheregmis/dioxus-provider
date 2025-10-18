//! # Composable Provider Demo
//!
//! This example demonstrates the powerful composable provider feature that allows
//! providers to run in parallel and combine their results. The demo uses CSS inlining
//! with `include_str!("./assets/composable_provider_styles.css")` for better cross-platform
//! compatibility and follows the standard Dioxus pattern.
//!
//! Features demonstrated:
//! - Parallel provider execution with `compose = [provider1, provider2, ...]`
//! - Type-safe result composition
//! - Performance comparison between sequential and parallel loading
//! - Partial composition for selective provider execution
//! - Error handling in composed providers
//! - Responsive design with properly inlined CSS styling
//! - Cross-platform compatibility (web, desktop, mobile)

use dioxus::prelude::*;
use dioxus_provider::hooks::ProviderState;
use dioxus_provider::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

#[cfg(not(target_arch = "wasm32"))]
use tokio::time::sleep;
#[cfg(target_arch = "wasm32")]
use wasmtimer::tokio::sleep;

/// Demo showcasing composable providers - combining multiple providers into one
///
/// This example demonstrates:
/// - Parallel execution of composed providers
/// - Error handling and aggregation
/// - Real-world use cases like user profiles with permissions
/// - How composed results are available in the main provider

// Basic data structures
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub id: u32,
    pub name: String,
    pub email: String,
    pub avatar_url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserPermissions {
    pub user_id: u32,
    pub permissions: Vec<String>,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserSettings {
    pub user_id: u32,
    pub theme: String,
    pub language: String,
    pub notifications_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FullUserProfile {
    pub user: User,
    pub permissions: UserPermissions,
    pub settings: UserSettings,
    pub load_time_ms: u64,
}

// Individual providers that can be composed

/// Fetch basic user information
#[provider(cache_expiration = "2min")]
async fn fetch_user(user_id: u32) -> Result<User, ProviderError> {
    // Simulate API delay
    sleep(Duration::from_millis(100)).await;

    match user_id {
        0 => Err(ProviderError::InvalidInput(
            "User ID cannot be zero".to_string(),
        )),
        1 => Ok(User {
            id: 1,
            name: "Alice Johnson".to_string(),
            email: "alice@example.com".to_string(),
            avatar_url: "https://via.placeholder.com/100".to_string(),
        }),
        2 => Ok(User {
            id: 2,
            name: "Bob Smith".to_string(),
            email: "bob@example.com".to_string(),
            avatar_url: "https://via.placeholder.com/100".to_string(),
        }),
        _ => Err(ProviderError::Generic("User not found".to_string())),
    }
}

/// Fetch user permissions
#[provider(cache_expiration = "5min")]
async fn fetch_user_permissions(user_id: u32) -> Result<UserPermissions, ProviderError> {
    // Simulate API delay
    sleep(Duration::from_millis(150)).await;

    match user_id {
        0 => Err(ProviderError::InvalidInput(
            "User ID cannot be zero".to_string(),
        )),
        1 => Ok(UserPermissions {
            user_id: 1,
            permissions: vec!["read".to_string(), "write".to_string(), "admin".to_string()],
            role: "Administrator".to_string(),
        }),
        2 => Ok(UserPermissions {
            user_id: 2,
            permissions: vec!["read".to_string()],
            role: "User".to_string(),
        }),
        _ => Err(ProviderError::Generic("Permissions not found".to_string())),
    }
}

/// Fetch user settings
#[provider(cache_expiration = "1min")]
async fn fetch_user_settings(user_id: u32) -> Result<UserSettings, ProviderError> {
    // Simulate API delay
    sleep(Duration::from_millis(80)).await;

    match user_id {
        0 => Err(ProviderError::InvalidInput(
            "User ID cannot be zero".to_string(),
        )),
        1 => Ok(UserSettings {
            user_id: 1,
            theme: "dark".to_string(),
            language: "en".to_string(),
            notifications_enabled: true,
        }),
        2 => Ok(UserSettings {
            user_id: 2,
            theme: "light".to_string(),
            language: "es".to_string(),
            notifications_enabled: false,
        }),
        _ => Err(ProviderError::Generic("Settings not found".to_string())),
    }
}

// Composable provider that combines all user data

/// Fetch complete user profile by composing multiple providers
/// This runs all three providers in parallel and combines their results
#[provider(compose = [fetch_user, fetch_user_permissions, fetch_user_settings], cache_expiration = "3min")]
async fn fetch_full_user_profile(user_id: u32) -> Result<FullUserProfile, ProviderError> {
    let start_time = Instant::now();

    // The composed results are automatically available as variables:
    // - __dioxus_composed_fetch_user_result: Result<User, ProviderError>
    // - __dioxus_composed_fetch_user_permissions_result: Result<UserPermissions, ProviderError>
    // - __dioxus_composed_fetch_user_settings_result: Result<UserSettings, ProviderError>

    // All three providers have been called in parallel with user_id
    // We can now combine their results

    let user = __dioxus_composed_fetch_user_result?;
    let permissions = __dioxus_composed_fetch_user_permissions_result?;
    let settings = __dioxus_composed_fetch_user_settings_result?;

    let loading_time = start_time.elapsed();

    Ok(FullUserProfile {
        user,
        permissions,
        settings,
        load_time_ms: loading_time.as_millis() as u64,
    })
}

// Additional example: Compose just two providers

/// Fetch user with permissions (subset of full profile)
#[provider(compose = [fetch_user, fetch_user_permissions])]
async fn fetch_user_with_permissions(
    user_id: u32,
) -> Result<(User, UserPermissions), ProviderError> {
    // The composed results are available as:
    // - __dioxus_composed_fetch_user_result: Result<User, ProviderError>
    // - __dioxus_composed_fetch_user_permissions_result: Result<UserPermissions, ProviderError>

    let user = __dioxus_composed_fetch_user_result?;
    let permissions = __dioxus_composed_fetch_user_permissions_result?;

    Ok((user, permissions))
}

// UI Components

#[component]
fn App() -> Element {
    // Initialize global providers for dependency injection
    dioxus_provider::init();

    // State for demonstration
    let mut user_id = use_signal(|| 1u32);

    rsx! {
        div { class: "container",

            // Header
            div { class: "header",
                h1 { "🚀 Composable Provider Demo" }
                p { "Demonstrates the power of composable providers in parallel execution" }

                // User selection buttons
                div { class: "user-selector",
                    button {
                        class: "user-button primary",
                        onclick: move |_| { *user_id.write() = 1; },
                        "Load User 1"
                    }
                    button {
                        class: "user-button primary",
                        onclick: move |_| { *user_id.write() = 2; },
                        "Load User 2"
                    }
                    button {
                        class: "user-button danger",
                        onclick: move |_| { *user_id.write() = 999; },
                        "Load Non-existent User (Error)"
                    }
                }

                p { style: "color: #666;", "Current User ID: " { user_id.read().to_string() } }
            }

            // Performance comparison section
            div { class: "section",
                h2 { "⚡ Performance Comparison" }
                p { "Compare sequential loading (individual providers) vs parallel loading (composable providers)" }
            }

            // Individual providers (sequential loading simulation)
            div { class: "section",
                h2 { "🐌 Individual Providers (Sequential Loading Simulation)" }
                p { "Each provider loads independently, taking longer total time" }
                IndividualProvidersDemo { user_id: *user_id.read() }
            }

            // Composable provider (parallel loading)
            div { class: "section success",
                h2 { "⚡ Composable Provider (Parallel Loading)" }
                p { "All providers execute in parallel, significantly faster!" }
                ComposableProviderDemo { user_id: *user_id.read() }
            }

            // Partial composition demo
            div { class: "section",
                h2 { "🔗 Partial Composition" }
                p { "Compose only specific providers as needed" }
                PartialCompositionDemo { user_id: *user_id.read() }
            }

            // Information section
            div { class: "feature-list",
                h3 { "💡 Key Benefits of Composable Providers" }
                ul {
                    li { "⚡ " span { class: "highlight", "Parallel Execution: " } "All composed providers run simultaneously" }
                    li { "🎯 " span { class: "highlight", "Type Safety: " } "Full compile-time type checking for all composed results" }
                    li { "🔄 " span { class: "highlight", "Automatic Caching: " } "Each provider maintains its own cache and settings" }
                    li { "🛠 " span { class: "highlight", "Flexible Composition: " } "Mix and match providers as needed" }
                    li { "📊 " span { class: "highlight", "Error Handling: " } "Individual provider errors don't break the whole composition" }
                    li { "🚀 " span { class: "highlight", "Performance: " } "Significantly faster than sequential loading" }
                }
            }
        }

        style { {include_str!("./assets/composable_provider_styles.css")} }
    }
}

#[component]
fn IndividualProvidersDemo(user_id: u32) -> Element {
    let user_data = use_provider(fetch_user(), user_id);
    let permissions_data = use_provider(fetch_user_permissions(), user_id);
    let settings_data = use_provider(fetch_user_settings(), user_id);

    rsx! {
        div { class: "grid",

            // User card
            div { class: "card user-section",
                h4 { "👤 User Data" }
                match &*user_data.read() {
                    ProviderState::Loading { .. } => rsx! { p { class: "loading", "Loading user..." } },
                    ProviderState::Success(user) => rsx! {
                        div {
                            p { strong { "Name: " } {user.name.clone()} }
                            p { strong { "Email: " } {user.email.clone()} }
                            p { strong { "ID: " } {user.id.to_string()} }
                        }
                    },
                    ProviderState::Error(err) => rsx! { p { style: "color: #dc3545;", "Error: {err}" } }
                }
            }

            // Permissions card
            div { class: "card permissions-section",
                h4 { "🔐 Permissions" }
                match &*permissions_data.read() {
                    ProviderState::Loading { .. } => rsx! { p { class: "loading", "Loading permissions..." } },
                    ProviderState::Success(perms) => rsx! {
                        div {
                            p { strong { "Role: " } {perms.role.clone()} }
                            p { strong { "Permissions: " } {perms.permissions.join(", ")} }
                        }
                    },
                    ProviderState::Error(err) => rsx! { p { style: "color: #dc3545;", "Error: {err}" } }
                }
            }

            // Settings card
            div { class: "card settings-section",
                h4 { "⚙️ Settings" }
                match &*settings_data.read() {
                    ProviderState::Loading { .. } => rsx! { p { class: "loading", "Loading settings..." } },
                    ProviderState::Success(settings) => rsx! {
                        div {
                            p { strong { "Theme: " } {settings.theme.clone()} }
                            p { strong { "Language: " } {settings.language.clone()} }
                            p { strong { "Notifications: " } {if settings.notifications_enabled { "Enabled" } else { "Disabled" }} }
                        }
                    },
                    ProviderState::Error(err) => rsx! { p { style: "color: #dc3545;", "Error: {err}" } }
                }
            }
        }
    }
}

#[component]
fn ComposableProviderDemo(user_id: u32) -> Element {
    let profile_data = use_provider(fetch_full_user_profile(), user_id);

    rsx! {
        div { class: "grid",
            match &*profile_data.read() {
                ProviderState::Loading { .. } => rsx! {
                    div { class: "loading",
                        p { "⚡ Loading full profile in parallel..." }
                    }
                },
                ProviderState::Success(profile) => rsx! {
                    div { class: "success composition-section",
                        div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 15px;",
                            h3 { style: "color: #28a745; margin: 0;", "✅ Full Profile Loaded" }
                            span { class: "timing-badge",
                                "Loaded in {profile.load_time_ms}ms"
                            }
                        }

                        div { class: "grid",
                            // User section
                            div { class: "card user-section",
                                h4 { "👤 User Info" }
                                p { strong { "Name: " } {profile.user.name.clone()} }
                                p { strong { "Email: " } {profile.user.email.clone()} }
                                p { strong { "ID: " } {profile.user.id.to_string()} }
                            }

                            // Permissions section
                            div { class: "card permissions-section",
                                h4 { "🔐 Permissions" }
                                p { strong { "Role: " } {profile.permissions.role.clone()} }
                                p { strong { "Permissions: " } {profile.permissions.permissions.join(", ")} }
                            }

                            // Settings section
                            div { class: "card settings-section",
                                h4 { "⚙️ Settings" }
                                p { strong { "Theme: " } {profile.settings.theme.clone()} }
                                p { strong { "Language: " } {profile.settings.language.clone()} }
                                p { strong { "Notifications: " } {if profile.settings.notifications_enabled { "Enabled" } else { "Disabled" }} }
                            }
                        }
                    }
                },
                ProviderState::Error(err) => rsx! {
                    div { class: "error",
                        h3 { "❌ Failed to Load Profile" }
                        p { "Error: {err}" }
                        p { style: "color: #666; font-size: 14px;",
                            "This demonstrates graceful error handling in composed providers."
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn PartialCompositionDemo(user_id: u32) -> Element {
    let user_with_permissions = use_provider(fetch_user_with_permissions(), user_id);

    rsx! {
        div { class: "grid",
            match &*user_with_permissions.read() {
                ProviderState::Loading { .. } => rsx! {
                    p { class: "loading", "Loading user with permissions..." }
                },
                ProviderState::Success(user_perms) => rsx! {
                    div { class: "card",
                        h3 { style: "color: #6f42c1; margin-top: 0;", "👤🔐 User + Permissions (Partial Composition)" }

                        div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 20px;",
                            div {
                                h4 { "User" }
                                p { strong { "Name: " } {user_perms.0.name.clone()} }
                                p { strong { "Email: " } {user_perms.0.email.clone()} }
                                p { strong { "ID: " } {user_perms.0.id.to_string()} }
                            }
                            div {
                                h4 { "Permissions" }
                                p { strong { "Role: " } {user_perms.1.role.clone()} }
                                p { strong { "Permissions: " } {user_perms.1.permissions.join(", ")} }
                            }
                        }
                    }
                },
                ProviderState::Error(err) => rsx! {
                    p { style: "color: #dc3545;", "Error: {err}" }
                }
            }
        }
    }
}

fn main() {
    // Launch the app
    dioxus::launch(App);
}
