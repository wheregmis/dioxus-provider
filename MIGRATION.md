# Migration Guide: v0.0.x → v0.1.0

This guide will help you migrate your code from dioxus-provider v0.0.x to v0.1.0.

## Overview

Version 0.1.0 introduces several improvements and minor breaking changes focused on:
- Simplified initialization
- Unified cache APIs
- Consistent state types
- Better error handling
- Improved module organization

All deprecated APIs remain available but will show compiler warnings encouraging migration.

## Breaking Changes

### 1. Global Initialization (Recommended Update)

The initialization system has been consolidated for better developer experience.

**Before:**
```rust
use dioxus_provider::global::init_global_providers;
use dioxus_provider::injection::init_dependency_injection;

fn main() {
    init_global_providers();
    init_dependency_injection();  // If using DI
    dioxus::launch(app);
}
```

**After:**
```rust
use dioxus_provider;

fn main() {
    // Single init handles both providers and dependency injection
    dioxus_provider::init();
    dioxus::launch(app);
}
```

**Alternative (granular control):**
```rust
use dioxus_provider::ProviderConfig;

fn main() {
    // Only initialize providers (no DI)
    ProviderConfig::new().init();
    
    // Or with dependency injection
    ProviderConfig::new()
        .with_dependency_injection()
        .init();
        
    dioxus::launch(app);
}
```

### 2. Removed Functions

The following deprecated functions have been removed:

- `get_global_cache_panic()` → Use `get_global_cache()?` with error handling
- `get_global_refresh_registry_panic()` → Use `get_global_refresh_registry()?`

**Before:**
```rust
let cache = get_global_cache_panic();
```

**After:**
```rust
let cache = get_global_cache().expect("Providers not initialized");
// Or handle the error properly
let cache = get_global_cache()?;
```

## API Improvements (Non-Breaking)

### 3. Unified Cache API

The cache now provides a unified `get_with_options()` method for more flexible retrieval.

**Before:**
```rust
// Old separate methods
let data = cache.get_with_expiration::<MyType>("key", Some(Duration::from_secs(60)));
let (data, is_stale) = cache.get_with_staleness::<MyType>("key", stale_time, expiration);
```

**After:**
```rust
use dioxus_provider::cache::CacheGetOptions;

// Unified options API
let options = CacheGetOptions::new()
    .with_expiration(Duration::from_secs(300))
    .with_stale_time(Duration::from_secs(60));
    
let result = cache.get_with_options::<MyType>("key", options);
if let Some(result) = result {
    println!("Data: {:?}, Stale: {}", result.data, result.is_stale);
}

// Simple get still works for common cases
let data = cache.get::<MyType>("key");
```

### 4. AsyncState Trait

Both `ProviderState` and `MutationState` now implement a unified `AsyncState` trait.

**Before:**
```rust
// Duplicate helper methods on each type
match &*mutation_state.read() {
    MutationState::Loading => { /* ... */ },
    MutationState::Success(data) => { /* ... */ },
    // ...
}
```

**After:**
```rust
use dioxus_provider::prelude::AsyncState;

// Same API for both provider and mutation states
if state.read().is_loading() {
    // Works for both ProviderState and MutationState
}

let data = state.read().data();  // Unified method
```

### 5. Enhanced MutationContext

New helper methods make working with mutations easier:

```rust
use dioxus_provider::prelude::*;

#[mutation(invalidates = [fetch_items])]
async fn add_item(
    item: String,
    ctx: MutationContext<Vec<String>, String>,
) -> Result<Vec<String>, String> {
    // NEW: map_or_else provides a default
    Ok(ctx.map_or_else(
        || vec![item.clone()],  // Default if no cache
        |items| items.push(item.clone())
    ))
}

#[mutation(invalidates = [fetch_counter])]
async fn increment(
    ctx: MutationContext<i32, String>,
) -> Result<i32, String> {
    // NEW: update_in_place is more explicit
    ctx.update_in_place(|count| *count += 1)
        .ok_or_else(|| "No counter available".to_string())
}

// NEW: Helper methods
ctx.has_data();   // Check if cache has data
ctx.has_error();  // Check if cache has error
```

## Module Organization Changes

The internal module structure has been reorganized for clarity:

- `src/hooks/main.rs` → `src/hooks/provider.rs` (internal change, no API impact)
- Helper modules moved to `src/hooks/internal/` (internal change, no API impact)

These changes don't affect public APIs.

## Feature Flags

### New: `tracing` Feature (Enabled by Default)

Logging is now optional via the `tracing` feature flag, which is enabled by default:

```toml
# Default: tracing enabled with emojis
[dependencies]
dioxus-provider = "0.1"

# Disable all logging (smaller binary, better performance)
[dependencies]
dioxus-provider = { version = "0.1", default-features = false }

# Enable tracing with plain text (no emojis)  
[dependencies]
dioxus-provider = { version = "0.1", features = ["plain-logs"] }
```

**Benefits:**
- **Performance**: Disabling tracing can improve performance in production
- **Binary Size**: Reduces binary size by removing tracing dependencies
- **Flexibility**: Choose emoji-decorated or plain text logging

## Deprecation Timeline

- **v0.1.0**: Old APIs deprecated with warnings
- **v0.2.0** (planned): Deprecated APIs will be removed

## Quick Migration Checklist

- [ ] Replace `init_global_providers()` and `init_dependency_injection()` with `dioxus_provider::init()`
- [ ] Remove calls to removed `get_global_cache_panic()` and similar functions
- [ ] (Optional) Update cache operations to use `get_with_options()` for better clarity
- [ ] (Optional) Add `AsyncState` trait usage for cleaner state checking
- [ ] (Optional) Use new `MutationContext` helper methods for cleaner mutation code
- [ ] Run `cargo check` and fix any deprecation warnings
- [ ] Test your application to ensure everything works correctly

## Need Help?

- Check the [examples](./examples/) for updated usage patterns
- Review the API documentation for detailed information
- Open an issue on GitHub if you encounter migration problems

## Example: Full Migration

**Before (v0.0.x):**
```rust
use dioxus::prelude::*;
use dioxus_provider::{
    global::init_global_providers,
    injection::init_dependency_injection,
    prelude::*,
};

fn main() {
    init_global_providers();
    init_dependency_injection();
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let user = use_provider(fetch_user(), (1,));
    
    match &*user.read() {
        ProviderState::Loading { .. } => rsx! { "Loading..." },
        ProviderState::Success(data) => rsx! { "{data}" },
        ProviderState::Error(e) => rsx! { "Error: {e}" },
    }
}
```

**After (v0.1.0):**
```rust
use dioxus::prelude::*;
use dioxus_provider::prelude::*;

fn main() {
    // Simplified initialization
    dioxus_provider::init();
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let user = use_provider(fetch_user(), (1,));
    
    // Same pattern matching works, or use new helpers
    if user.read().is_loading() {
        return rsx! { "Loading..." };
    }
    
    match user.read().data() {
        Some(data) => rsx! { "{data}" },
        None => rsx! { "Error or loading" },
    }
}
```

