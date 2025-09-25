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
//! ## How it works:
//! 1. Provider loads initial data (only place data is defined!)
//! 2. User clicks delete
//! 3. Library gets current cached data and passes it to:
//!    - optimistic_updates_with_current() → immediate UI update
//!    - mutate_with_current() → server mutation (also works with current data!)
//! 4. UI updates instantly (item disappears)
//! 5. Server mutation runs in background (using same current data)
//! 6. If successful: nothing changes (optimistic result was correct)
//! 7. If failed: library rolls back + shows error

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
    // Add some logging to understand when this reloads
    println!("🔄 [LOAD_ITEMS] Provider is executing - returning fresh data");

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

    async fn mutate(&self, _id: u64) -> Result<Self::Output, Self::Error> {
        // Fallback implementation - should not be called in practice
        tokio::time::sleep(Duration::from_millis(1000)).await;
        Err(ItemError::Other("No current data available".to_string()))
    }

    async fn mutate_with_current(
        &self,
        id: u64,
        current_data: Option<&Result<Self::Output, Self::Error>>,
    ) -> Result<Self::Output, Self::Error> {
        // Simulate network delay
        println!("🌐 [MUTATION] Starting server mutation for item {}", id);

        // Add specific debugging for item 3
        if id == 3 {
            println!("⚠️ [DEBUG] Processing item 3 - this might hang!");
        }

        tokio::time::sleep(Duration::from_millis(1000)).await;
        println!("🌐 [MUTATION] Sleep completed for item {}", id);

        // Work with the actual current cached data!
        if let Some(Ok(current_items)) = current_data {
            println!(
                "🌐 [MUTATION] Working with {} current items for item {}",
                current_items.len(),
                id
            );

            // Debug the filtering process
            let items_before_filter: Vec<_> = current_items.iter().map(|item| item.id).collect();
            println!(
                "🌐 [MUTATION] Items before filter: {:?}",
                items_before_filter
            );

            // Remove the item from current state
            let updated_items: Vec<Item> = current_items
                .iter()
                .filter(|item| {
                    let keep = item.id != id;
                    if !keep {
                        println!("🌐 [MUTATION] Filtering out item {}", item.id);
                    }
                    keep
                })
                .cloned()
                .collect();

            let items_after_filter: Vec<_> = updated_items.iter().map(|item| item.id).collect();
            println!("🌐 [MUTATION] Items after filter: {:?}", items_after_filter);

            println!(
                "🌐 [MUTATION] Mutation complete, returning {} items for item {}",
                updated_items.len(),
                id
            );

            // Simulate potential server failure (uncomment to test rollback)
            // if id == 2 { return Err(ItemError::Other("Server error".to_string())); }

            Ok(updated_items)
        } else {
            println!("❌ [MUTATION] No current data to work with for item {}", id);
            Err(ItemError::Other("No current data to work with".to_string()))
        }
    }

    fn invalidates(&self) -> Vec<String> {
        vec![provider_cache_key_simple(load_items())]
    }

    fn optimistic_updates_with_current(
        &self,
        input: &u64,
        current_data: Option<&Result<Self::Output, Self::Error>>,
    ) -> Vec<(String, Result<Self::Output, Self::Error>)> {
        // This is the EFFICIENT way to do optimistic mutations!
        // We get the current cached data and modify it instead of duplicating
        let id_to_delete = *input;

        println!(
            "🔄 [OPTIMISTIC] Deleting item {} optimistically",
            id_to_delete
        );

        if let Some(Ok(current_items)) = current_data {
            println!(
                "🔄 [OPTIMISTIC] Current items count: {}",
                current_items.len()
            );
            // Filter out the deleted item from the current data
            let optimistic_result: Vec<Item> = current_items
                .iter()
                .filter(|item| item.id != id_to_delete)
                .cloned()
                .collect();

            println!(
                "🔄 [OPTIMISTIC] After deletion, items count: {}",
                optimistic_result.len()
            );

            vec![(
                provider_cache_key_simple(load_items()),
                Ok(optimistic_result),
            )]
        } else {
            println!("❌ [OPTIMISTIC] No current data available for optimistic update");
            // No current data available, return empty (could fallback to invalidation)
            vec![]
        }
    }
}

/// Item component with delete button  
/// Notice: NO data duplication anywhere! The mutation works with cached data just like optimistic updates
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
