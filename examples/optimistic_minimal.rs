//! Minimal optimistic mutation demo for `dioxus-provider`.
//!
//! ## What this example shows
//! - `#[provider]` defines the read-only data source with zero boilerplate.
//! - `#[mutation(..., optimistic = ...)]` auto-applies the optimistic update and passes
//!   the mutated data directly to your function - no duplication, no manual context checks!
//! - Multi-argument optimistic mutations are fully supported.
//! - `use_mutation` automatically detects optimistic updates and only reports an error if the
//!   server rejects the change.
//!
//! ## Try it
//! 1. Run `cargo run --example optimistic_minimal`.
//! 2. Delete any item. It disappears immediately thanks to the auto-applied optimistic update.
//! 3. Update any item's name. It changes immediately with the multi-arg optimistic mutation.
//! 4. Toggle "Simulate Errors" to see automatic rollback when mutations fail!
//!
//! The rest of this file stays intentionally small so you can focus on the macro APIs.

use dioxus::prelude::*;
use dioxus_provider::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::time::{Duration, sleep};

/// Global flag to simulate errors for demonstration purposes
static SIMULATE_ERRORS: AtomicBool = AtomicBool::new(false);

/// Simple item to demonstrate optimistic mutations
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    pub id: u64,
    pub name: String,
}

/// Simple error type
#[derive(Debug, thiserror::Error, Clone, PartialEq)]
pub enum ItemError {
    #[error("Item not found")]
    NotFound,
    #[error("Other error: {0}")]
    Other(String),
}

/// Provider for loading items
#[provider]
pub async fn load_items() -> Result<Vec<Item>, ItemError> {
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

/// Delete an item - optimistic update is auto-applied!
#[mutation(
    invalidates = [load_items],
    optimistic = |items: &mut Vec<Item>, id: &u64| items.retain(|item| item.id != *id)
)]
pub async fn delete_item(_id: u64, items: Vec<Item>) -> Result<Vec<Item>, ItemError> {
    sleep(Duration::from_millis(1000)).await;

    // Simulate server error for demonstration
    if SIMULATE_ERRORS.load(Ordering::Relaxed) {
        return Err(ItemError::Other(
            "Simulated server error - deletion rejected!".to_string(),
        ));
    }

    // In a real app, you'd persist to a backend here
    // The optimistic update is already applied to `items`
    Ok(items)
}

/// Update an item's name - demonstrates multi-argument optimistic mutation
#[mutation(
    invalidates = [load_items],
    optimistic = |items: &mut Vec<Item>, id: &u64, new_name: &String| {
        if let Some(item) = items.iter_mut().find(|i| i.id == *id) {
            item.name = new_name.clone();
        }
    }
)]
pub async fn update_item(
    _id: u64,
    _new_name: String,
    items: Vec<Item>,
) -> Result<Vec<Item>, ItemError> {
    sleep(Duration::from_millis(1000)).await;

    // Simulate server error for demonstration
    if SIMULATE_ERRORS.load(Ordering::Relaxed) {
        return Err(ItemError::Other(
            "Simulated server error - update rejected!".to_string(),
        ));
    }

    // In a real app, you'd persist to a backend here
    // The optimistic update is already applied to `items`
    Ok(items)
}

/// Item component with delete and update buttons demonstrating optimistic mutations
#[component]
pub fn ItemCard(item: Item) -> Element {
    let (delete_state, delete_item) = use_mutation(delete_item());
    let (update_state, update_item) = use_mutation(update_item());
    let mut new_name = use_signal(|| item.name.clone());
    let item_id = item.id;

    let on_delete = move |_| {
        delete_item(item_id);
    };

    let on_update = move |_| {
        let name = new_name.read().clone();
        update_item((item_id, name));
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
                button { onclick: on_update, "Update" }
                button { onclick: on_delete, "Delete" }
            }
            if let MutationState::Error(err) = &*delete_state.read() {
                div { style: "color: red; margin-top: 5px;", "Delete error: {err}" }
            }
            if let MutationState::Error(err) = &*update_state.read() {
                div { style: "color: red; margin-top: 5px;", "Update error: {err}" }
            }
        }
    }
}

/// Items list component
#[component]
pub fn ItemsList() -> Element {
    let items = use_provider(load_items(), ());

    rsx! {
        div {
            h2 { "Items List" }
            match &*items.read() {
                State::Loading { .. } => rsx! {
                    div { "Loading..." }
                },
                State::Error(err) => rsx! {
                    div { "Error: {err}" }
                },
                State::Success(items) => {
                    if items.is_empty() {
                        rsx! {
                            div { "No items" }
                        }
                    } else {
                        rsx! {
                            div {
                                for item in items {
                                    ItemCard { item: item.clone() }
                                }
                            }
                        }
                    }
                }
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
            h1 { "Optimistic Mutations Demo" }

            div {
                style: "background: #f0f0f0; padding: 15px; margin: 15px 0; border-radius: 5px;",
                p { "This demo shows both single-arg and multi-arg optimistic mutations:" }
                ul {
                    li { "Delete: Single-arg optimistic mutation - item disappears INSTANTLY" }
                    li { "Update: Multi-arg optimistic mutation - name changes INSTANTLY" }
                }
                p {
                    style: "margin-bottom: 0;",
                    "No loading states, no waiting - just immediate feedback!"
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
                            "✓ Simulate Errors (mutations will fail and rollback)"
                        } else {
                            "☐ Simulate Errors (enable to see automatic rollback)"
                        }
                    }
                }
                if simulate_errors() {
                    p {
                        style: "margin: 10px 0 0 0; color: #856404;",
                        "⚠️ Try deleting or updating items - they will appear to change, then rollback with an error!"
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
