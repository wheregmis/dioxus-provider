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
- **Declarative `#[provider]` Macro**: Define data sources with a simple attribute. The macro handles all the complex boilerplate for you.
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

### Structured Error Handling ✨ NEW!
- **Rich Error Types**: Comprehensive error hierarchy with `ProviderError`, `UserError`, `ApiError`, and `DatabaseError`.
- **Actionable Error Messages**: Context-rich error information for better debugging and user feedback.
- **Error Chaining**: Automatic error conversion and chaining using `#[from]` attributes.
- **Backward Compatibility**: Seamless integration with existing String-based error handling.

### Mutation System
- **Manual Implementation Pattern**: Define data mutations using simple struct implementations.
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

At the entry point of your application, call `init_global_providers()` once. This sets up the global cache that all providers will use.

```rust,no_run
use dioxus_provider::global::init_global_providers;
use dioxus::prelude::*;

fn main() {
    // This is required for all provider hooks to work
    init_global_providers();
    launch(app);
}

fn app() -> Element {
    rsx! { /* Your app content */ }
}
```

### 2. Create a Provider

A "provider" is a function that fetches or computes a piece of data. Use the `#[provider]` attribute to turn any `async` function into a data source that can be used throughout your app.

```rust,no_run
use dioxus_provider::prelude::*;
use std::time::Duration;

// This could be an API call, database query, etc.
#[provider]
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
use dioxus::prelude::ReadableExt;

#[provider]
async fn get_server_message() -> Result<String, String> {
    Ok("Hello from the server!".to_string())
}

#[component]
fn App() -> Element {
    // Use the provider hook to get the data
    let message = use_provider(get_server_message(), ());

    rsx! {
        div {
            h1 { "Dioxus Provider Demo" }
            // Pattern match on the state to render UI
            match &*message.read() {
                State::Loading { .. } => rsx! { div { "Loading..." } },
                State::Success(data) => rsx! { div { "Server says: {data}" } },
                State::Error(err) => rsx! { div { "Error: {err}" } },
            }
        }
    }
}
```

## Mutations: Modifying Data with Automatic Cache Management

The mutation system allows you to define data modification operations that automatically invalidate related provider caches, ensuring your UI stays in sync with server state.

### 1. Basic Mutations (Macro-Based)

Create mutations using the `#[mutation]` attribute. Mutations automatically invalidate specified provider caches when they succeed.

```rust,ignore
use dioxus_provider::prelude::*;

#[provider]
async fn fetch_todos() -> Result<Vec<Todo>, String> { todo!() }

#[derive(Clone, PartialEq)]
struct Todo;

// Define a mutation that invalidates the todo list when successful
#[mutation(invalidates = [fetch_todos])]
async fn add_todo(title: String) -> Result<Todo, String> {
    Ok(Todo)
}
```

### 2. Optimistic Updates

For better UX, add an `optimistic` parameter to your mutation that updates the UI immediately and rolls back on failure:

```rust,ignore
use dioxus_provider::prelude::*;

#[provider]
async fn fetch_todos() -> Result<Vec<Todo>, String> { todo!() }

#[derive(Clone, PartialEq)]
struct Todo {
    id: u32,
    completed: bool,
}

#[mutation(
    invalidates = [fetch_todos],
    optimistic = |todos: &mut Vec<Todo>, id: &u32| {
        if let Some(todo) = todos.iter_mut().find(|t| t.id == *id) {
            todo.completed = !todo.completed;
        }
    }
)]
async fn toggle_todo(id: u32) -> Result<Vec<Todo>, String> {
    Ok(vec![])
}
```

### 3. Multiple Cache Invalidation

Mutations can invalidate multiple provider caches at once:

```rust,ignore
use dioxus_provider::prelude::*;

#[provider]
async fn fetch_todos() -> Result<Vec<Todo>, String> { todo!() }

#[provider]
async fn fetch_stats() -> Result<String, String> { todo!() }

#[derive(Clone, PartialEq)]
struct Todo;

#[mutation(invalidates = [fetch_todos, fetch_stats])]
async fn remove_todo(id: u32) -> Result<(), String> {
    Ok(())
}
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

#[provider]
async fn fetch_user(_id: u32) -> Result<User, ProviderError> { Ok(User) }

#[provider]
async fn fetch_permissions(_id: u32) -> Result<Permissions, ProviderError> { Ok(Permissions) }

#[provider]
async fn fetch_settings(_id: u32) -> Result<Settings, ProviderError> { Ok(Settings) }

// These providers will run in parallel
#[provider(compose = [fetch_user, fetch_permissions, fetch_settings])]
async fn fetch_complete_profile(user_id: u32) -> Result<UserProfile, ProviderError> {
    // The macro generates parallel execution
    let user = fetch_user(user_id).await?;
    let permissions = fetch_permissions(user_id).await?;
    let settings = fetch_settings(user_id).await?;
    
    Ok(UserProfile { user, permissions, settings })
}
```

### Structured Error Handling

Rich, actionable error types for better error handling:

```rust,ignore
use dioxus_provider::prelude::*;

#[derive(Clone, PartialEq)]
struct User { is_suspended: bool }

impl User {
    fn is_suspended(&self) -> bool { self.is_suspended }
}

#[derive(thiserror::Error, Debug, Clone, PartialEq)]
pub enum UserError {
    #[error("Validation failed: {field} - {reason}")]
    ValidationFailed { field: String, reason: String },
    #[error("User is suspended: {reason}")]
    Suspended { reason: String },
    #[error("User not found: {id}")]
    NotFound { id: u32 },
}

#[provider]
async fn fetch_user_data(id: u32) -> Result<User, UserError> {
    if id == 0 {
        return Err(UserError::ValidationFailed {
            field: "id".to_string(),
            reason: "ID cannot be zero".to_string(),
        });
    }
    Ok(User { is_suspended: false })
}
```

## Advanced Usage

### Parameterized Providers

Providers can take arguments to fetch dynamic data. For example, fetching a user by their ID. The cache is keyed by the arguments, so `fetch_user(1)` and `fetch_user(2)` are cached separately.

```rust,no_run
use dioxus::prelude::*;
use dioxus_provider::prelude::*;
use dioxus::prelude::ReadableExt;

#[provider]
async fn fetch_user(user_id: u32) -> Result<String, String> {
    Ok(format!("User data for ID: {}", user_id))
}

#[component]
fn UserProfile(user_id: u32) -> Element {
    // Pass arguments as a tuple
    let user = use_provider(fetch_user(), (user_id,));

    match &*user.read() {
        State::Success(data) => rsx!{ div { "{data}" } },
        // ... other states
        _ => rsx!{ div { "Loading user..." } }
    }
}
```

### Caching Strategies

#### Stale-While-Revalidate (SWR)

`stale_time` serves cached (stale) data first, then re-fetches in the background. This provides a great UX by showing data immediately.

```rust,ignore
use dioxus_provider::prelude::*;

#[provider(stale_time = "10s")]
async fn get_dashboard_data() -> Result<String, String> {
    Ok("Dashboard data".to_string())
}
```

#### Cache Expiration (TTL)

`cache_expiration` evicts data from the cache after a time-to-live (TTL). The next request will show a loading state while it re-fetches.

```rust,ignore
use dioxus_provider::prelude::*;

// This data will be removed from cache after 5 minutes of inactivity
#[provider(cache_expiration = "5m")]
async fn get_analytics() -> Result<String, String> {
    Ok("Analytics report".to_string())
}
```

### Manual Cache Invalidation

You can manually invalidate a provider's cache to force a re-fetch.

```rust,ignore
use dioxus::prelude::*;
use dioxus_provider::prelude::*;

#[provider]
async fn fetch_user(_id: u32) -> Result<String, String> {
    Ok("User".to_string())
}

#[component]
fn UserDashboard() -> Element {
    let user_data = use_provider(fetch_user(), (1,));
    let invalidate_user = use_invalidate_provider(fetch_user(), (1,));

    rsx! {
        button {
            onclick: move |_| invalidate_user(),
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

## State Combinators

`State` now supports combinator methods for ergonomic state transformations:

```rust
use dioxus_provider::prelude::*;

let state: State<u32, String> = State::Success(42);
let mapped = state.clone().map(|v| v.to_string()); // State<String, String>
let state2: State<u32, String> = State::Success(42);
let mapped_err = state2.clone().map_err(|e| format!("error: {e}"));
let state3: State<u32, String> = State::Success(42);
let chained = state3.and_then(|v| if v > 0 { State::Success(v * 2) } else { State::Error("zero".into()) });
```

See the API docs for more details.

## Examples Gallery

Explore the full power of `dioxus-provider` with these real-world, ready-to-run examples in the [`examples/`](./examples/) directory:

| Example | Description |
|---------|-------------|
| [`comprehensive_demo.rs`](./examples/comprehensive_demo.rs) | **All-in-one showcase**: Demonstrates global providers, interval refresh, SWR, cache expiration, error handling, parameterized providers, and more. |
| [`cache_expiration_demo.rs`](./examples/cache_expiration_demo.rs) | **Cache Expiration**: Shows how data is evicted and re-fetched after TTL, with manual invalidation and cache hit/miss indicators. |
| [`swr_demo.rs`](./examples/swr_demo.rs) | **Stale-While-Revalidate (SWR)**: Instant data serving from cache, background revalidation, and manual refresh. |
| [`interval_refresh_demo.rs`](./examples/interval_refresh_demo.rs) | **Interval Refresh**: Automatic background data updates at configurable intervals. |
| [`composable_provider_demo.rs`](./examples/composable_provider_demo.rs) | **Composable Providers**: Parallel provider execution, type-safe result composition, and error aggregation. |
| [`dependency_injection_demo.rs`](./examples/dependency_injection_demo.rs) | **Dependency Injection**: Macro-based DI for API clients, databases, and more. |
| [`structured_errors_demo.rs`](./examples/structured_errors_demo.rs) | **Structured Error Handling**: Rich error types, actionable messages, and error chaining. |
| [`counter_mutation_demo.rs`](./examples/counter_mutation_demo.rs) | **Mutations**: Counter with provider invalidation and mutation state tracking. |
| [`cache_expiration_test.rs`](./examples/cache_expiration_test.rs) | **Cache Expiration Test**: Verifies that cache expiration triggers loading and refetch. |
| [`suspense_demo.rs`](./examples/suspense_demo.rs) | **Suspense Integration**: Shows how to use Dioxus SuspenseBoundary with async providers. |
| [`provider_state_combinators.rs`](./examples/provider_state_combinators.rs) | **ProviderState Combinators**: Practical use of `.map`, `.map_err`, and `.and_then` for ergonomic state transformations in UI. |

> **Tip:**
> Run any example with  
> `cargo run --example <example_name>`  
> (e.g., `cargo run --example swr_demo`)

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

#[provider]
async fn fetch_user(_id: u32) -> Result<String, String> { Ok("User".to_string()) }

#[component]
fn AnimatedUserCard(user_id: u32) -> Element {
    // Data fetching with dioxus-provider
    let user_data = use_provider(fetch_user(), (user_id,));
    
    match &*user_data.read() {
        State::Success(user) => rsx! {
            div {
                "{user}"
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