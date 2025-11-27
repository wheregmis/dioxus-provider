# Dioxus Provider

[![Crates.io](https://img.shields.io/crates/v/dioxus-provider.svg)](https://crates.io/crates/dioxus-provider)
[![Docs.rs](https://docs.rs/dioxus-provider/badge.svg)](https://docs.rs/dioxus-provider)

> **⚠️ In Development**  
> This library is currently in active development. The API may change before the first stable release. Please check the [changelog](./CHANGELOG.md) for the latest updates and breaking changes.

**Effortless, powerful, and scalable data fetching and caching for Dioxus applications, inspired by [Riverpod for Flutter](https://riverpod.dev/).**

`dioxus-provider` provides a simple yet robust way to manage data fetching, handle asynchronous operations, and cache data with minimal boilerplate. It is designed to feel native to Dioxus, integrating seamlessly with its component model and hooks system.

## Key Features

### Data Fetching & Caching
- **Global Provider System**: Manage application-wide data without nesting context providers. Simplifies component architecture and avoids "provider hell."
- **Simple Async Functions**: Define data sources using standard Rust `async` functions. No macros required!
- **Intelligent Caching Strategies**:
    - **Stale-While-Revalidate (SWR)**: Serve stale data instantly while fetching fresh data in the background for a lightning-fast user experience.
    - **Time-to-Live (TTL) Cache Expiration**: Automatically evict cached data after a configured duration.
- **Automatic Refresh**: Keep data fresh with interval-based background refetching.
- **Parameterized Queries**: Create providers that depend on dynamic arguments (e.g., fetching user data by ID).

### Composable Providers ✨ NEW!
- **Parallel Execution**: Run multiple providers simultaneously with `compose = [provider1, provider2, ...]` for significant performance gains.
- **Type-Safe Composition**: Automatic result combination with compile-time safety guarantees.
- **Flexible Composition**: Compose any subset of providers based on your specific needs.
- **Error Aggregation**: Intelligent error handling across composed providers with proper error propagation.

### Structured Error Handling

`dioxus-provider` no longer ships domain-specific errors. Instead, define your own domain enums (with `thiserror`) and use them in providers. For example:

```rust,ignore
use dioxus_provider::prelude::*;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq)]
struct UserProfile;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum UserError {
    #[error("User not found: {id}")]
    NotFound { id: u32 },
    #[error("User is suspended: {reason}")]
    Suspended { reason: String },
}

async fn fetch_user_profile(user_id: u32) -> Result<UserProfile, UserError> {
    // ...
    Ok(UserProfile)
}
```

- **Bring Your Own Domain Errors**: Model failures with custom enums (via `thiserror`) that fit your backends.
- **ProviderError for Framework Issues**: Use the built-in `ProviderError` only when reporting configuration/DI/cache failures.
- **Actionable Error Messages**: Context-rich error information for better debugging and user feedback.
- **String Compatibility**: Seamless integration with existing String-based error handling.

### Mutation System
- **Simple Async Mutations**: Define data mutations using standard `async` functions.
- **Optimistic Updates**: Immediate UI feedback with automatic rollback on failure.
- **Smart Cache Invalidation**: Automatically refresh related providers after successful mutations.
- **Mutation State Tracking**: Built-in loading, success, and error states for mutations.
- **Type-Safe Parameters**: Support for no parameters, single parameters, and multiple parameters (tuples).

### Developer Experience
- **Manual Cache Control**: Hooks to manually invalidate cached data or clear the entire cache.
- **Cross-Platform by Default**: Works seamlessly on both Desktop and Web (WASM).
- **Minimal Boilerplate**: Get started in minutes with intuitive hooks and macros.
- **Type Safety**: Full TypeScript-level type safety with Rust's type system.

## Installation

Add `dioxus-provider` to your `Cargo.toml`:

```toml
[dependencies]
dioxus-provider = "0.0.1" # Replace with the latest version
```

## Getting Started

### 1. Initialize Global Providers

At the entry point of your application, call `init()` once. This sets up the global cache, refresh registry, and optional dependency injection.

```rust,no_run
use dioxus_provider::global::init;
use dioxus::prelude::*;

fn main() {
    // This is required for all provider hooks to work
    init().unwrap();
    launch(app);
}

fn app() -> Element {
    rsx! { /* Your app content */ }
}
```

### 2. Create a Provider

A "provider" is simply an `async` function that fetches or computes a piece of data. No macros or special traits are required!

```rust,no_run
use dioxus_provider::prelude::*;
use std::time::Duration;

// This could be an API call, database query, etc.
async fn get_server_message() -> Result<String, String> {
    // Simulate a network request
    tokio::time::sleep(Duration::from_secs(1)).await;
    Ok("Hello from the server!".to_string())
}
```

### 3. Use the Provider in a Component

Use the `use_provider` hook to read data from a provider. Dioxus will automatically re-render your component when the data changes (e.g., when the `async` function completes).

The hook returns a `Signal<State<T, E>>`, which can be in one of three states: `Loading`, `Success(T)`, or `Error(E)`.

```rust,no_run
use dioxus::prelude::*;
use dioxus_provider::prelude::*;

async fn get_server_message() -> Result<String, String> {
    Ok("Hello from the server!".to_string())
}

#[component]
fn App() -> Element {
    // 1. Create the provider handle
    let mut message_provider = use_provider(get_server_message);

    // 2. Trigger the fetch (e.g., on mount)
    use_effect(move || {
        message_provider.call();
    });

    rsx! {
        div {
            h1 { "Dioxus Provider Demo" }
            
            // 3. Access state and data
    match message_provider.state() {
        State::Pending => rsx! { div { "Loading..." } },
        State::Ready => {
            if let Some(Ok(data)) = message_provider.value() {
                rsx! { div { "Server says: {data}" } }
            } else {
                rsx! { div { "No data" } }
            }
        },
        State::Error => {
            if let Some(err) = message_provider.error() {
                rsx! { div { "Error: {err}" } }
            } else {
                rsx! { div { "Unknown error" } }
            }
        },
        _ => rsx! { div { "Idle" } }
    }
        }
    }
}
```

## Mutations: Modifying Data with Automatic Cache Management

The mutation system allows you to define data modification operations that automatically invalidate related provider caches, ensuring your UI stays in sync with server state.

### 1. Basic Mutations

Use `use_mutation` with an `async` function. You can chain `.invalidates(provider)` to automatically refresh data when the mutation succeeds.

```rust,ignore
use dioxus_provider::prelude::*;

async fn fetch_todos() -> Result<Vec<Todo>, String> { todo!() }

#[derive(Clone, PartialEq)]
struct Todo;

async fn add_todo(title: String) -> Result<Todo, String> {
    Ok(Todo)
}

// In component:
// let mutation = use_mutation(add_todo).invalidates(fetch_todos);
```

### 2. Optimistic Updates

For better UX, you can configure optimistic updates that update the UI immediately and roll back on failure. Note: The optimistic update API is currently being refined for the hook-based approach.

### 3. Multiple Cache Invalidation

Mutations can invalidate multiple provider caches at once:

```rust,ignore
use dioxus_provider::prelude::*;

async fn fetch_todos() -> Result<Vec<Todo>, String> { todo!() }

async fn fetch_stats() -> Result<String, String> { todo!() }

#[derive(Clone, PartialEq)]
struct Todo;

async fn remove_todo(id: u32) -> Result<(), String> {
    Ok(())
}

// In component:
// let mutation = use_mutation(remove_todo)
//     .invalidates(fetch_todos)
//     .invalidates(fetch_stats);
```

## New Features in Latest Release

### Composable Providers: Parallel Data Loading

Run multiple providers simultaneously for better performance:

```rust,ignore
use dioxus_provider::prelude::*;

#[derive(Clone, PartialEq)]
struct User;
#[derive(Clone, PartialEq)]
struct Permissions;
#[derive(Clone, PartialEq)]
struct Settings;
#[derive(Clone, PartialEq)]
struct UserProfile { user: User, permissions: Permissions, settings: Settings }

async fn fetch_user(_id: u32) -> Result<User, ProviderError> { Ok(User) }

async fn fetch_permissions(_id: u32) -> Result<Permissions, ProviderError> { Ok(Permissions) }

async fn fetch_settings(_id: u32) -> Result<Settings, ProviderError> { Ok(Settings) }

// Composition is now done via hooks or manual async aggregation
// (New composition API coming soon)
```

### Structured Error Handling

Define your own domain-specific error types (e.g., with `thiserror`) and use them in providers. See the earlier section for a complete example; built-in `ApiError`/`DatabaseError` types have been removed to keep the core library lean.

## Advanced Usage

### Parameterized Providers

Providers can take arguments to fetch dynamic data. For example, fetching a user by their ID. The cache is keyed by the arguments, so `fetch_user(1)` and `fetch_user(2)` are cached separately.

```rust,no_run
use dioxus::prelude::*;
use dioxus_provider::prelude::*;

async fn fetch_user(user_id: u32) -> Result<String, String> {
    Ok(format!("User data for ID: {}", user_id))
}

#[component]
fn UserProfile(user_id: u32) -> Element {
    // 1. Create provider
    let mut user_provider = use_provider(fetch_user);

    // 2. Fetch when user_id changes
    use_effect(move || {
        user_provider.call(user_id);
    });

    rsx! {
        div {
            if user_provider.pending() {
                "Loading user..."
            } else if let Some(Ok(data)) = user_provider.value() {
                "{data}"
            }
        }
    }
}
```

### Caching Strategies

#### Stale-While-Revalidate (SWR)

`stale_time` serves cached (stale) data first, then re-fetches in the background. This provides a great UX by showing data immediately.

```rust,ignore
use dioxus_provider::prelude::*;

async fn get_dashboard_data() -> Result<String, String> {
    Ok("Dashboard data".to_string())
}

// In component:
// let mut data = use_provider(get_dashboard_data)
//     .stale_time(Duration::from_secs(10));
// data.call();
```

#### Cache Expiration (TTL)

`cache_expiration` evicts data from the cache after a time-to-live (TTL). The next request will show a loading state while it re-fetches.

```rust,ignore
use dioxus_provider::prelude::*;

async fn get_analytics() -> Result<String, String> {
    Ok("Analytics report".to_string())
}

// In component:
// let mut data = use_provider(get_analytics)
//     .cache_expiration(Duration::from_secs(300));
// data.call();
```

### Manual Cache Invalidation

You can manually invalidate a provider's cache to force a re-fetch.

```rust,ignore
use dioxus::prelude::*;
use dioxus_provider::prelude::*;

async fn fetch_user(_id: u32) -> Result<String, String> {
    Ok("User".to_string())
}

#[component]
fn UserDashboard() -> Element {
    let mut user_provider = use_provider(fetch_user);
    
    use_effect(move || { user_provider.call(1); });

    rsx! {
        button {
            onclick: move |_| user_provider.invalidate(),
            "Refresh User"
        }
    }
}
```

To clear the entire global cache for all providers:

```rust,ignore
use dioxus::prelude::*;
use dioxus_provider::prelude::*;

#[component]
fn App() -> Element {
    let clear_cache = use_clear_provider_cache();
    rsx! {
        button {
            onclick: move |_| clear_cache(),
            "Clear Cache"
        }
    }
}
```


### Provider Lifecycle Controls

You can fine-tune cache behaviour per provider using lifecycle controls:

```rust,ignore
// Configure lifecycle in the component
let mut dashboard = use_provider(fetch_dashboard)
    .interval(Duration::from_secs(4))
    .stale_time(Duration::from_secs(6))
    .cache_expiration(Duration::from_secs(30));

dashboard.call(user_id);
```

If omitted, no background tasks run – providers simply fetch when requested. Combine interval polling, SWR, and TTL as needed.

For example implementations, see `examples/comprehensive_demo.rs`, which configures interval refresh, SWR, and cache expiration on different providers.



## Examples Gallery

Explore the full power of `dioxus-provider` with these real-world, ready-to-run examples in the [`examples/`](./examples/) directory:
> `cargo run --example <example_name>`  
> (e.g., `cargo run --example comprehensive_demo`)

## Ecosystem & Alternatives

### dioxus-query: For Complex, Type-Safe Data Management

For more complex applications requiring advanced type safety, sophisticated caching strategies, and enterprise-grade data management, we highly recommend **[dioxus-query](https://github.com/marc2332/dioxus-query)** by [Marc](https://github.com/marc2332).

**dioxus-query** is a mature, production-ready library that provides:

- **Advanced Type Safety**: Compile-time guarantees for complex data relationships
- **Sophisticated Caching**: Multi-level caching with intelligent invalidation strategies
- **Query Dependencies**: Automatic dependency tracking and cascading updates
- **Optimistic Updates**: Immediate UI updates with rollback on failure
- **Background Synchronization**: Advanced background sync with conflict resolution
- **Enterprise Features**: Built-in support for complex data patterns and edge cases

**When to choose dioxus-query:**
- Large-scale applications with complex data requirements
- Teams requiring maximum type safety and compile-time guarantees
- Applications with sophisticated caching and synchronization needs
- Enterprise applications where data consistency is critical

**When to choose dioxus-provider:**
- Smaller to medium applications
- Quick prototyping and development
- Teams new to Dioxus data management
- Applications where simplicity and ease of use are priorities

### dioxus-motion: For Smooth Animations and Transitions

Looking to add beautiful animations to your Dioxus application? Check out **[dioxus-motion](https://github.com/wheregmis/dioxus-motion)** - a lightweight, cross-platform animation library also built by me.

**dioxus-motion** provides:

- **Cross-Platform Animations**: Works seamlessly on web, desktop, and mobile
- **Declarative Animation API**: Write animations as data, not imperative code
- **Page Transitions**: Smooth route transitions with `AnimatedOutlet`
- **Spring Physics**: Natural, physics-based animations
- **Custom Easing**: Extensive easing function support
- **Type-Safe Animations**: Compile-time animation safety
- **Extensible**: Implement `Animatable` trait for custom types

**Perfect combination:**
- Use **dioxus-provider** for data fetching and caching
- Use **dioxus-motion** for smooth UI animations and transitions
- Both libraries work together seamlessly in the same application

```rust,ignore
// Example: Combining dioxus-provider with dioxus-motion
use dioxus::prelude::*;
use dioxus_provider::prelude::*;

async fn fetch_user(_id: u32) -> Result<String, String> { Ok("User".to_string()) }

#[component]
fn AnimatedUserCard(user_id: u32) -> Element {
    // Data fetching with dioxus-provider
    let mut user_provider = use_provider(fetch_user);
    use_effect(move || { user_provider.call(user_id); });
    
    match user_provider.state() {
        State::Ready => {
             if let Some(Ok(user)) = user_provider.value() {
                rsx! {
                    div { "{user}" }
                }
             } else {
                 rsx! { div { "Error" } }
             }
        },
        _ => rsx! { div { "Loading..." } }
    }
}
```

### Acknowledgment

Special thanks to [Marc](https://github.com/marc2332) for creating the excellent **dioxus-query** library, which has been a significant inspiration for this project. Marc's work on dioxus-query has helped establish best practices for data management in the Dioxus ecosystem, and we encourage users to explore both libraries to find the best fit for their specific use case.

## Contributing

Contributions are welcome! Please feel free to open an issue or submit a pull request.

## License

This project is licensed under the MIT License.
