//! Minimal Optimistic Mutations Example for dioxus-provider
//!
//! This demonstrates the NEW optimistic mutations system:
//!
//! ✅ INSTANT UI updates - no loading states, no waiting
//! ✅ Automatic rollback on failure
//! ✅ Clean, simple API - just implement optimistic_updates()
//! ✅ Library handles all the complexity
//!
//! ## How it works:
//! 1. User clicks delete
//! 2. Library immediately applies optimistic_updates() to cache
//! 3. UI updates instantly (item disappears)
//! 4. Server mutation runs in background
//! 5. If successful: nothing changes (optimistic result was correct)
//! 6. If failed: library rolls back + shows error

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
    // Return some mock data
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

/// Mutation: Delete an item
#[mutation(invalidates = [load_items])]
pub async fn delete_item(id: u64) -> Result<Vec<Item>, ItemError> {
    sleep(Duration::from_millis(1000)).await; // Simulate delay

    // Simulate the deletion and return new list
    let items = vec![
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
    ];

    let filtered: Vec<Item> = items.into_iter().filter(|item| item.id != id).collect();
    Ok(filtered)
}

/// Optimistic delete mutation
#[derive(Clone, PartialEq)]
pub struct OptimisticDeleteMutation;

impl Mutation<u64> for OptimisticDeleteMutation {
    type Output = Vec<Item>;
    type Error = ItemError;

    async fn mutate(&self, id: u64) -> Result<Self::Output, Self::Error> {
        // Simulate the actual mutation
        tokio::time::sleep(Duration::from_millis(1000)).await;

        // Simulate the deletion and return new list
        let items = vec![
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
        ];

        let filtered: Vec<Item> = items.into_iter().filter(|item| item.id != id).collect();
        Ok(filtered)
    }

    fn invalidates(&self) -> Vec<String> {
        vec![provider_cache_key_simple(load_items())]
    }

    fn optimistic_updates(&self, input: &u64) -> Vec<(String, Result<Self::Output, Self::Error>)> {
        // This is the NEW way to do optimistic mutations!
        // Return the expected result immediately for instant UI updates
        let id_to_delete = *input;

        // Calculate the optimistic result (items without the deleted one)
        let current_items = vec![
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
        ];

        let optimistic_result: Vec<Item> = current_items
            .into_iter()
            .filter(|item| item.id != id_to_delete)
            .collect();

        vec![(
            provider_cache_key_simple(load_items()),
            Ok(optimistic_result),
        )]
    }
}

/// Item component with delete button  
/// This demonstrates the NEW simplified pattern using the enhanced library
#[component]
pub fn ItemCard(item: Item) -> Element {
    // Use the optimistic mutation hook - the library handles everything automatically!
    let (delete_state, delete_item) = use_optimistic_mutation(OptimisticDeleteMutation);

    let item_id = item.id;
    let on_delete = move |_| {
        // That's it! The library will:
        // 1. Apply optimistic updates immediately (from optimistic_updates method)
        // 2. Show the loading state
        // 3. Run the actual mutation in the background
        // 4. If successful: keep the optimistic result
        // 5. If failed: rollback to original state automatically
        delete_item(item_id);
    };

    rsx! {
        div {
            span { "{item.name}" }
            button { onclick: on_delete, "Delete" }
            // Only show errors - no loading state for truly optimistic UX!
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
                ProviderState::Loading { .. } => rsx!(div { "Loading..." }),
                ProviderState::Error(err) => rsx!(div { "Error: {err}" }),
                ProviderState::Success(items) => {
                    if items.is_empty() {
                        rsx!(div { "No items" })
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
