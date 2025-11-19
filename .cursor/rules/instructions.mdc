---
description: You are an expert in Rust, and a crazy engineer to build user friendly data fetching and caching library.
alwaysApply: false
---

# Copilot Instructions for dioxus-provider

## Project Overview

**dioxus-provider** is a Rust data fetching and caching library for Dioxus (UI framework). It provides:
- Global provider system for application-wide data management
- Intelligent caching with SWR (Stale-While-Revalidate), TTL expiration, and LRU eviction
- Declarative `#[provider]` and `#[mutation]` macros for minimal boilerplate
- Optimistic updates and automatic cache invalidation
- Cross-platform support (Desktop + WASM)

## Architecture

### Core Components

1. **Global Singleton System** (`src/global.rs`)
   - `ProviderCache`: Type-erased global cache storing all provider results
   - `RefreshRegistry`: Tracks active refresh tasks
   - Initialized once via `dioxus_provider::init()` at app startup
   - Uses `OnceLock` for thread-safe lazy initialization

2. **Macro System** (`dioxus-provider-macros/src/lib.rs`)
   - `#[provider]` macro: Converts async functions into cacheable providers with optional caching strategies
   - `#[mutation]` macro: Defines mutations with automatic cache invalidation
   - Supports: `interval`, `cache_expiration`, `stale_time` (providers), `invalidates`, `optimistic` (mutations)
   - Duration strings use humantime format: `"1m"`, `"5m"`, `"30s"`, etc.

3. **Hook System** (`src/hooks/provider.rs`)
   - `use_provider()`: Main hook to consume provider data (returns `Signal<State<T, E>>`)
   - `use_mutation()`: Execute mutations and track state
   - `use_invalidate_provider()`: Manually invalidate specific provider caches
   - `use_clear_provider_cache()`: Clear entire cache
   - `use_provider_cache()`: Inspect cache directly

4. **Caching Strategy** (`src/cache.rs`)
   - Transparent type-erasure via `Any` trait
   - LRU eviction with configurable size limits (platform-specific defaults)
   - Per-entry staleness tracking for SWR revalidation
   - Access statistics for cache introspection

5. **State Management** (`src/state.rs`, `src/mutation.rs`)
   - `State<T, E>`: Enum with `Loading`, `Success(T)`, `Error(E)` variants
   - `MutationState<T, E>`: Similar to `State`, used for mutations
   - Both implement `AsyncState` trait for consistent interface

### Data Flow

```
App Start → init() → Global Cache + Refresh Registry initialized
            ↓
    use_provider(provider_fn, params)
            ↓
    Check cache for key(params) → HIT: return cached, schedule revalidation if stale
                                ↓
                              MISS: fetch, store in cache, return
            ↓
    Cache expiration/invalidation → RefreshRegistry triggers revalidation
            ↓
    Component re-renders via Dioxus signals
```

## Critical Patterns

### Provider Definition

```rust
// Simple provider (no parameters)
#[provider(cache_expiration = "5m", stale_time = "1m")]
async fn fetch_todos() -> Result<Vec<Todo>, String> {
    // Fetches fresh data after 1min, serves stale data until 5min expiration
    api::get_todos().await
}

// Parameterized provider (supports scalar, tuple, or custom types)
#[provider]
async fn fetch_user(user_id: u32) -> Result<User, String> {
    api::get_user(user_id).await
}

// Composed providers (parallel execution)
#[provider(compose = [fetch_user, fetch_posts])]
async fn user_dashboard(user_id: u32) -> Result<DashboardData, String> {
    // Both providers execute in parallel, results combined
}
```

### Mutation Definition

```rust
// Automatic cache invalidation
#[mutation(invalidates = [fetch_todos])]
async fn add_todo(title: String) -> Result<Todo, String> {
    api::add_todo(title).await
}

// With optimistic updates
#[mutation(
    invalidates = [fetch_todos],
    optimistic = |todos| { /* update todos list optimistically */ }
)]
async fn toggle_todo(id: u32) -> Result<(), String> {
    api::toggle_todo(id).await
}
```

### Component Usage

```rust
#[component]
fn TodoApp() -> Element {
    // Consume provider
    let todos = use_provider(fetch_todos(), ());
    
    // Use mutation
    let (add_state, add_mutation) = use_mutation(add_todo());

    rsx! {
        match &*todos.read() {
            State::Loading => rsx! { div { "Loading..." } },
            State::Success(items) => rsx! { /* render items */ },
            State::Error(e) => rsx! { div { "Error: {e}" } },
        }
        
        button {
            onclick: move |_| {
                add_mutation("New Todo".to_string());
            },
            "Add Todo"
        }
    }
}
```

## Type Constraints

All types used with providers must satisfy trait bounds defined in `src/types.rs`:

- **Parameter types**: `Clone + PartialEq + Hash + Debug + Send + Sync + 'static`
  - Use `#[derive(Clone, PartialEq, Eq, Hash, Debug)]` for custom types
- **Output types**: `Clone + PartialEq + Send + Sync + 'static`
  - Use `#[derive(Clone, PartialEq)]` for custom types
- **Error types**: Implement `Clone + PartialEq + Debug + Display + Send + Sync + 'static`
  - Use `thiserror` or `anyhow` crates for structured errors

## Error Handling

Structured error types in `src/errors.rs`:
- `ProviderError`: General provider failures (InvalidInput, Network, ExternalService, etc.)
- `UserError`, `ApiError`, `DatabaseError`: Domain-specific errors
- Implement custom errors with `#[derive(...)]` from `thiserror`

```rust
#[derive(thiserror::Error, Debug, Clone, PartialEq)]
pub enum FetchError {
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}
```

## Developer Workflows

### Build & Test
```bash
cargo check                    # Quick validation
cargo build                    # Debug build
cargo build --release         # Production build
cargo test                     # Run all tests
cargo test --doc             # Run doc tests
```

### Examples
Run any example with context (shows cache behavior, SWR revalidation, etc.):
```bash
cargo run --example todo_app
cargo run --example swr_demo
cargo run --example dependency_injection_demo
```

### Workspace Structure
- `src/`: Core library
  - `hooks/internal/`: Private implementations (cache_mgmt.rs, swr.rs, tasks.rs)
  - `hooks/provider.rs`: Main `use_provider()` hook
- `dioxus-provider-macros/`: Procedural macros for `#[provider]` and `#[mutation]`
- `examples/`: Runnable demos illustrating all features

## Key Implementation Details

### Cache Key Generation
- Cache keys are computed as `(provider_fn_id, params_hash)`
- Parameter types must implement `Hash` for this to work
- Tuple parameters `()` generate simple keys for non-parameterized providers

### Task Management
- Refresh tasks spawn via `dioxus::spawn()` (platform-agnostic)
- Interval tasks use `tokio::time::interval` on desktop, browser timers on WASM
- All tasks are stored in `RefreshRegistry` for cleanup

### Platform Differences
- **Desktop**: Uses full `tokio` runtime with multi-threading
- **WASM**: Uses `wasm-bindgen-futures` and browser async APIs
- Abstracted in `src/platform.rs` for unified interface

## Testing Considerations

- Use `cargo test --lib` for unit tests
- Macro expansion is transparent; test generated code by inspecting `.cargo/` output if needed
- Examples serve as integration tests; verify they compile and run
- Manual cache introspection via `use_provider_cache()` hook for debugging

## Common Pitfalls

1. **Forgetting `init()`**: Must call `dioxus_provider::init()` before using any hooks
2. **Parameter traits**: Custom param types must derive `Clone, PartialEq, Eq, Hash, Debug`
3. **Cache key collisions**: Ensure params uniquely identify provider calls; use custom types if scalars conflict
4. **Circular invalidations**: Avoid mutations invalidating providers that trigger those mutations
5. **Mutation state**: Always check `MutationState` before acting (don't ignore `Loading`/`Error`)

## References

- **Main traits**: `src/hooks/provider.rs::Provider`, `src/mutation.rs::Mutation`
- **Macro specs**: `dioxus-provider-macros/src/lib.rs`
- **Examples**: See `examples/` directory for all patterns in action
- **Macros used**: `#[provider(...)]`, `#[mutation(...)]` from prelude
