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
//! 4. Uncomment the simulated error inside mutations to see the automatic rollback.
//!
//! The rest of this file stays intentionally small so you can focus on the macro APIs.

use dioxus::prelude::*;
use dioxus_provider::prelude::*;
use serde::{Deserialize, Serialize};
use tokio::time::{Duration, sleep};

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
                ProviderState::Loading { .. } => rsx! {
                    div { "Loading..." }
                },
                ProviderState::Error(err) => rsx! {
                    div { "Error: {err}" }
                },
                ProviderState::Success(items) => {
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
    rsx! {
        div {
            h1 { "Optimistic Mutations Demo" }
            p { "This demo shows both single-arg and multi-arg optimistic mutations:" }
            ul {
                li { "Delete: Single-arg optimistic mutation - item disappears INSTANTLY" }
                li { "Update: Multi-arg optimistic mutation - name changes INSTANTLY" }
            }
            p { "No loading states, no waiting - just immediate feedback!" }
            p { "If the server fails, changes will rollback with an error message." }
            ItemsList {}
        }
    }
}

fn main() {
    let _ = dioxus_provider::init();
    dioxus::launch(App);
}
