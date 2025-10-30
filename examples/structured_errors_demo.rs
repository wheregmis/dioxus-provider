//! Structured Error Handling Demo
//!
//! This example demonstrates the new structured error handling capabilities

use dioxus::prelude::*;
use dioxus_provider::prelude::*;
use std::time::Duration;

// Cross-platform sleep
#[cfg(not(target_arch = "wasm32"))]
use tokio::time::sleep;
#[cfg(target_arch = "wasm32")]
use wasmtimer::tokio::sleep;

// Mock types
#[derive(Debug, Clone, PartialEq)]
pub struct User {
    pub id: u32,
    pub name: String,
    pub email: String,
    pub status: UserStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UserStatus {
    Active,
    Suspended,
    Deleted,
}

// Mock API client
#[derive(Clone)]
pub struct ApiClient {
    pub base_url: String,
}

impl ApiClient {
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }

    pub async fn get_user(&self, id: u32) -> ApiResult<User> {
        sleep(Duration::from_millis(100)).await;

        match id {
            0 => Err(ApiError::HttpStatus {
                status: 400,
                message: "Invalid user ID".to_string(),
            }),
            1 => Ok(User {
                id: 1,
                name: "John Doe".to_string(),
                email: "john@example.com".to_string(),
                status: UserStatus::Active,
            }),
            2 => Ok(User {
                id: 2,
                name: "Jane Smith".to_string(),
                email: "jane@example.com".to_string(),
                status: UserStatus::Suspended,
            }),
            3 => Ok(User {
                id: 3,
                name: "Bob Wilson".to_string(),
                email: "bob@example.com".to_string(),
                status: UserStatus::Deleted,
            }),
            404 => Err(ApiError::HttpStatus {
                status: 404,
                message: "User not found".to_string(),
            }),
            _ => Err(ApiError::EndpointNotFound {
                endpoint: format!("/users/{id}"),
            }),
        }
    }
}

// Provider demonstrating ProviderError
#[provider]
async fn fetch_user_basic(user_id: u32) -> Result<User, ProviderError> {
    if user_id == 0 {
        return Err(ProviderError::InvalidInput(
            "User ID must be greater than 0".to_string(),
        ));
    }

    sleep(Duration::from_millis(100)).await;

    match user_id {
        1 => Ok(User {
            id: 1,
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            status: UserStatus::Active,
        }),
        _ => Err(ProviderError::ExternalService {
            service: "UserAPI".to_string(),
            error: "User not found".to_string(),
        }),
    }
}

// Provider demonstrating UserError with dependency injection
#[provider]
async fn fetch_user_with_validation(user_id: u32) -> Result<User, UserError> {
    let api_client = inject::<ApiClient>().map_err(UserError::Provider)?;

    if user_id == 0 {
        return Err(UserError::ValidationFailed {
            field: "user_id".to_string(),
            reason: "Cannot be zero".to_string(),
        });
    }

    let user = api_client
        .get_user(user_id)
        .await
        .map_err(|api_err| match api_err {
            ApiError::HttpStatus { status: 404, .. } => UserError::NotFound { id: user_id },
            other => UserError::Provider(ProviderError::ExternalService {
                service: "UserAPI".to_string(),
                error: other.to_string(),
            }),
        })?;

    match user.status {
        UserStatus::Deleted => Err(UserError::Deleted { id: user_id }),
        UserStatus::Suspended => Err(UserError::Suspended {
            reason: "Account temporarily suspended".to_string(),
        }),
        UserStatus::Active => Ok(user),
    }
}

#[component]
fn UserProfile(user_id: u32) -> Element {
    let user_data = use_provider(fetch_user_with_validation(), user_id);

    rsx! {
        div { class: "max-w-2xl mx-auto p-6",
            h1 { class: "text-3xl font-bold mb-6",
                "User Profile (ID: {user_id})"
            }

            match &*user_data.read() {
                State::Loading { .. } => rsx! {
                    div { class: "text-blue-500", "Loading user..." }
                },
                State::Success(user) => rsx! {
                    div { class: "bg-white rounded-lg shadow p-6",
                        h2 { class: "text-xl font-semibold mb-4", "User Information" }
                        p { "Name: {user.name}" }
                        p { "Email: {user.email}" }
                        p { "Status: {user.status:?}" }
                    }
                },
                State::Error(error) => rsx! {
                    div { class: "bg-red-50 border border-red-200 rounded p-4",
                        h3 { class: "text-red-800 font-medium", "Error" }
                        p { class: "text-red-700", "{error}" }

                        if error.to_string().contains("not found") {
                            p { class: "text-red-600 italic mt-2",
                                "User {user_id} does not exist."
                            }
                        } else if error.to_string().contains("suspended") {
                            p { class: "text-yellow-600 italic mt-2",
                                "This account has been suspended."
                            }
                        }
                    }
                },
            }
        }
    }
}

#[component]
fn ErrorTestPanel(user_id: Signal<u32>) -> Element {
    rsx! {
        div { class: "max-w-md mx-auto p-4 bg-gray-100 rounded",
            h2 { class: "text-lg font-semibold mb-4", "Test Different Errors" }
            p { class: "text-sm text-gray-600 mb-4",
                "Current user ID: {user_id()}"
            }

            div { class: "grid grid-cols-2 gap-2",
                button {
                    class: "px-3 py-2 bg-green-500 text-white rounded",
                    onclick: move |_| user_id.set(1),
                    "Success (ID: 1)"
                }
                button {
                    class: "px-3 py-2 bg-yellow-500 text-white rounded",
                    onclick: move |_| user_id.set(2),
                    "Suspended (ID: 2)"
                }
                button {
                    class: "px-3 py-2 bg-red-500 text-white rounded",
                    onclick: move |_| user_id.set(3),
                    "Deleted (ID: 3)"
                }
                button {
                    class: "px-3 py-2 bg-purple-500 text-white rounded",
                    onclick: move |_| user_id.set(404),
                    "Not Found (ID: 404)"
                }
            }
        }
    }
}

#[component]
fn App() -> Element {
    let user_id = use_signal(|| 1u32);

    rsx! {
        div { class: "min-h-screen bg-gray-50 py-8",
            div { class: "max-w-4xl mx-auto space-y-8",
                div { class: "text-center",
                    h1 { class: "text-4xl font-bold mb-2",
                        "Structured Error Handling Demo"
                    }
                    p { class: "text-gray-600",
                        "Demonstrating typed error handling"
                    }
                }

                ErrorTestPanel { user_id }
                UserProfile { user_id: user_id() }

                div { class: "bg-blue-50 border border-blue-200 rounded p-6",
                    h2 { class: "text-lg font-semibold mb-3",
                        "Error Types Demonstrated"
                    }
                    ul { class: "space-y-1 text-sm",
                        li { "• ProviderError - General provider issues" }
                        li { "• UserError - User-specific operations" }
                        li { "• ApiError - HTTP/API related errors" }
                        li { "• DatabaseError - Database operations" }
                    }
                }
            }
        }
    }
}

fn main() {
    // Initialize provider system with dependency injection
    let _ = dioxus_provider::init();

    register_dependency(ApiClient::new("https://api.example.com".to_string())).unwrap();

    dioxus::launch(App);
}
