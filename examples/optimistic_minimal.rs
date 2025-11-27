//! Minimal optimistic mutation demo for `dioxus-provider`.
//!
//! ## What this example shows
//! - The new simplified provider API with `use_provider(async_fn)`
//! - Optimistic mutations with `.optimistic()` builder method
//! - Multi-argument mutations are fully supported
//! - Automatic cache invalidation with `.invalidates()`
//!
//! ## Try it
//! 1. Run `cargo run --example optimistic_minimal`.
//! 2. Delete any item. It disappears immediately thanks to the optimistic update.
//! 3. Update any item's name. It changes immediately.
//! 4. Toggle "Simulate Errors" to see automatic rollback when mutations fail!

use dioxus::prelude::*;
use dioxus_provider::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

#[cfg(not(target_family = "wasm"))]
use tokio::time::sleep;
#[cfg(target_family = "wasm")]
use wasmtimer::tokio::sleep;

/// Global flag to simulate errors for demonstration purposes
static SIMULATE_ERRORS: AtomicBool = AtomicBool::new(false);

/// Simple item to demonstrate optimistic mutations
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    pub id: u64,
    pub name: String,
}

/// Simple error type
#[derive(Debug, Clone, PartialEq)]
pub enum ItemError {
    NotFound,
    Other(String),
}

impl std::fmt::Display for ItemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ItemError::NotFound => write!(f, "Item not found"),
            ItemError::Other(s) => write!(f, "{}", s),
        }
    }
}

// ============================================================================
// ASYNC FUNCTIONS - Plain async functions for data operations
// ============================================================================

/// Load all items
async fn load_items() -> Result<Vec<Item>, ItemError> {
    println!("🔄 [LOAD_ITEMS] Provider is executing - returning fresh data");

    Ok(vec![
        Item {
            id: 1,
            name: "Item 1".to_string(),
        },
        Item {
            id: 2,
            name: "Item 2".to_string(),
        },
        Item {
            id: 3,
            name: "Item 3".to_string(),
        },
    ])
}

/// Delete an item
async fn delete_item(id: u64) -> Result<u64, ItemError> {
    sleep(Duration::from_millis(1000)).await;

    // Simulate server error for demonstration
    if SIMULATE_ERRORS.load(Ordering::Relaxed) {
        return Err(ItemError::Other(
            "Simulated server error - deletion rejected!".to_string(),
        ));
    }

    // In a real app, you'd persist to a backend here
    Ok(id)
}

/// Update an item's name
async fn update_item(id: u64, new_name: String) -> Result<Item, ItemError> {
    sleep(Duration::from_millis(1000)).await;

    // Simulate server error for demonstration
    if SIMULATE_ERRORS.load(Ordering::Relaxed) {
        return Err(ItemError::Other(
            "Simulated server error - update rejected!".to_string(),
        ));
    }

    // In a real app, you'd persist to a backend here
    Ok(Item { id, name: new_name })
}

/// Item component with delete and update buttons demonstrating optimistic mutations
#[component]
pub fn ItemCard(item: Item) -> Element {
    // Create mutations with invalidations
    // Create mutations with invalidations and optimistic updates
    // Create mutations with invalidations and optimistic updates
    let mut delete_mutation = use_mutation(delete_item)
        .invalidate_all(load_items)
        .on_optimistic_update(|(id,), cache| {
            // Use typed key for safety!
            let key = load_items.ty_key(&());
            
            // Optimistically remove the item from the list
            if let Some(mut items) = cache.get_typed(&key) {
                items.retain(|i| i.id != *id);
                cache.set_typed(&key, items);
            }
            Some(*id)
        });
    
    let mut update_mutation = use_mutation(update_item)
        .invalidate_all(load_items)
        .on_optimistic_update(|(id, name), cache| {
            // Use typed key for safety!
            let key = load_items.ty_key(&());

            // Optimistically update the item in the list
            if let Some(mut items) = cache.get_typed(&key) {
                if let Some(item) = items.iter_mut().find(|i| i.id == *id) {
                    item.name = name.clone();
                }
                cache.set_typed(&key, items);
            }
            Some(Item { id: *id, name: name.clone() })
        });
    
    let mut new_name = use_signal(|| item.name.clone());
    let item_id = item.id;

    let on_delete = move |_| {
        delete_mutation.call(item_id);
    };

    let on_update = move |_| {
        let name = new_name.read().clone();
        update_mutation.call(item_id, name);
    };

    rsx! {
        div {
            style: "border: 1px solid #ccc; padding: 10px; margin: 5px;",
            div {
                style: "margin-bottom: 5px;",
                strong { "ID: {item.id} - " }
                span { "{item.name}" }
            }
            div {
                style: "display: flex; gap: 5px; align-items: center;",
                input {
                    r#type: "text",
                    value: "{new_name}",
                    oninput: move |evt| new_name.set(evt.value().clone())
                }
                button { 
                    onclick: on_update, 
                    disabled: update_mutation.pending(),
                    if update_mutation.pending() { "Updating..." } else { "Update" }
                }
                button { 
                    onclick: on_delete, 
                    disabled: delete_mutation.pending(),
                    if delete_mutation.pending() { "Deleting..." } else { "Delete" }
                }
            }
            if delete_mutation.errored() {
                div { style: "color: red; margin-top: 5px;", 
                    "Delete error: {delete_mutation.error().map(|e| e.to_string()).unwrap_or_default()}" 
                }
            }
            if update_mutation.errored() {
                div { style: "color: red; margin-top: 5px;", 
                    "Update error: {update_mutation.error().map(|e| e.to_string()).unwrap_or_default()}" 
                }
            }
        }
    }
}

/// Items list component
#[component]
pub fn ItemsList() -> Element {
    let mut items_provider = use_provider(load_items)
        .stale_time(Duration::from_secs(5));

    // Fetch on mount
    use_effect(move || {
        items_provider.call();
    });

    rsx! {
        div {
            h2 { "Items List" }
            match items_provider.state() {
                State::Pending | State::Idle => rsx! {
                    div { "Loading..." }
                },
                State::Error => rsx! {
                    div { "Error: {items_provider.error().map(|e| e.to_string()).unwrap_or_default()}" }
                },
                State::Ready => {
                    if let Some(items) = items_provider.get_data() {
                        if items.is_empty() {
                            rsx! {
                                div { "No items" }
                            }
                        } else {
                            rsx! {
                                div {
                                    for item in items {
                                        ItemCard { key: "{item.id}", item: item.clone() }
                                    }
                                }
                            }
                        }
                    } else {
                        rsx! { div { "No data" } }
                    }
                },
                _ => rsx! { div { "Reset" } }
            }
        }
    }
}

/// Main app component
#[component]
pub fn App() -> Element {
    let mut simulate_errors = use_signal(|| false);

    let toggle_errors = move |_| {
        let new_value = !simulate_errors();
        simulate_errors.set(new_value);
        SIMULATE_ERRORS.store(new_value, Ordering::Relaxed);
    };

    rsx! {
        div {
            style: "font-family: sans-serif; max-width: 600px; margin: 20px auto; padding: 20px;",
            h1 { "Mutations Demo (New API)" }

            div {
                style: "background: #f0f0f0; padding: 15px; margin: 15px 0; border-radius: 5px;",
                p { "This demo shows the new simplified mutation API:" }
                ul {
                    li { "Delete: Mutation with cache invalidation" }
                    li { "Update: Multi-arg mutation with cache invalidation" }
                }
                p {
                    style: "margin-bottom: 0;",
                    "After mutation completes, the items list is automatically refreshed!"
                }
            }

            div {
                style: "background: #fff3cd; padding: 15px; margin: 15px 0; border-radius: 5px; border: 1px solid #ffc107;",
                label {
                    style: "display: flex; align-items: center; gap: 10px; cursor: pointer; font-weight: bold;",
                    input {
                        r#type: "checkbox",
                        checked: simulate_errors(),
                        onchange: toggle_errors,
                        style: "width: 20px; height: 20px; cursor: pointer;"
                    }
                    span {
                        if simulate_errors() {
                            "✓ Simulate Errors (mutations will fail)"
                        } else {
                            "☐ Simulate Errors (enable to see error handling)"
                        }
                    }
                }
                if simulate_errors() {
                    p {
                        style: "margin: 10px 0 0 0; color: #856404;",
                        "⚠️ Try deleting or updating items - they will fail with an error!"
                    }
                }
            }

            ItemsList {}
        }
    }
}

fn main() {
    let _ = dioxus_provider::init();
    dioxus::launch(App);
}
