//! Minimal Optimistic Mutations Example for dioxus-provider
//!
//! This demonstrates the ZERO-DUPLICATION optimistic mutations system:
//!
//! ✅ INSTANT UI updates - no loading states, no waiting
//! ✅ Automatic rollback on failure  
//! ✅ ZERO data duplication - both optimistic updates AND mutations work with cached data
//! ✅ EFFICIENT API - library provides current cached data automatically
//! ✅ Library handles all the complexity
//!
//! ## How it works with the new macro API:
//! 1. Provider loads initial data (only place data is defined!)
//! 2. User clicks delete
//! 3. The `#[mutation]` macro supplies cached data through `MutationContext`
//! 4. The optimistic closure modifies cached data instantly, so the UI updates with zero delay
//! 5. The real mutation runs with the same cached data
//! 6. Success keeps the optimistic result; failure triggers automatic rollback

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

/// Delete an item while reusing the same optimistic closure for both optimistic and real updates.
#[mutation(
    invalidates = [load_items],
    optimistic = |items: &mut Vec<Item>, id: &u64| items.retain(|item| item.id != *id)
)]
pub async fn delete_item(
    id: u64,
    ctx: MutationContext<Vec<Item>, ItemError>,
) -> Result<Vec<Item>, ItemError> {
    println!("🌐 [MUTATION] Starting server mutation for item {}", id);
    if id == 3 {
        println!("⚠️ [DEBUG] Processing item 3 - extra logging enabled");
    }

    sleep(Duration::from_millis(1000)).await;
    println!("🌐 [MUTATION] Sleep completed for item {}", id);

    ctx.map_current(|items| {
        println!(
            "🌐 [MUTATION] Current items count before filter: {}",
            items.len()
        );
        let before: Vec<_> = items.iter().map(|item| item.id).collect();
        println!("🌐 [MUTATION] Items before filter: {:?}", before);
        items.retain(|item| item.id != id);
        let after: Vec<_> = items.iter().map(|item| item.id).collect();
        println!("🌐 [MUTATION] Items after filter: {:?}", after);
    })
    .ok_or_else(|| ItemError::Other("No current data to work with".to_string()))
}

/// Item component with delete button using the new macro-generated mutation
#[component]
pub fn ItemCard(item: Item) -> Element {
    let (delete_state, delete_item) = use_optimistic_mutation(delete_item());
    let item_id = item.id;

    let on_delete = move |_| {
        delete_item(item_id);
    };

    rsx! {
        div {
            span { "{item.name}" }
            button { onclick: on_delete, "Delete" }
            if let MutationState::Error(err) = &*delete_state.read() {
                span { style: "color: red; margin-left: 10px;", "Error: {err}" }
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
                ProviderState::Loading { .. } => rsx! { div { "Loading..." } },
                ProviderState::Error(err) => rsx! { div { "Error: {err}" } },
                ProviderState::Success(items) => {
                    if items.is_empty() {
                        rsx! { div { "No items" } }
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
            p { "Click delete and notice the item disappears INSTANTLY!" }
            p { "No loading states, no waiting - just immediate feedback." }
            p { "If the server fails, the item will reappear with an error message." }
            ItemsList {}
        }
    }
}

fn main() {
    let _ = dioxus_provider::global::init_global_providers();
    dioxus::launch(App);
}
