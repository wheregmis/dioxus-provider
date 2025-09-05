//! Comprehensive Todo App Example using dioxus-provider

use dioxus::prelude::Key;
use dioxus::prelude::*;
use dioxus_provider::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::fs;
use tokio::time::{Duration, sleep};

/// Represents a single todo item
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Todo {
    pub id: u64,
    pub title: String,
    pub completed: bool,
}

/// Filter for displaying todos
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    All,
    Active,
    Completed,
}

/// Error type for todo operations
#[derive(Debug, thiserror::Error)]
pub enum TodoError {
    #[error("IO error: {0}")]
    Io(Arc<std::io::Error>),
    #[error("JSON error: {0}")]
    Json(Arc<serde_json::Error>),
    #[error("Todo not found")]
    NotFound,
    #[error("Unknown error: {0}")]
    Other(String),
}

impl Clone for TodoError {
    fn clone(&self) -> Self {
        match self {
            TodoError::Io(e) => TodoError::Io(e.clone()),
            TodoError::Json(e) => TodoError::Json(e.clone()),
            TodoError::NotFound => TodoError::NotFound,
            TodoError::Other(s) => TodoError::Other(s.clone()),
        }
    }
}

impl PartialEq for TodoError {
    fn eq(&self, other: &Self) -> bool {
        use TodoError::*;
        match (self, other) {
            (NotFound, NotFound) => true,
            (Other(a), Other(b)) => a == b,
            (Io(a), Io(b)) => a.to_string() == b.to_string(),
            (Json(a), Json(b)) => a.to_string() == b.to_string(),
            _ => false,
        }
    }
}

const TODO_FILE: &str = "todos.json";

/// Load all todos from the persistent JSON file
async fn load_todos_from_file_async() -> Result<Vec<Todo>, TodoError> {
    match fs::read_to_string(TODO_FILE).await {
        Ok(data) => {
            let todos: Vec<Todo> =
                serde_json::from_str(&data).map_err(|e| TodoError::Json(Arc::new(e)))?;
            Ok(todos)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(vec![]),
        Err(e) => Err(TodoError::Io(Arc::new(e))),
    }
}

/// Provider for loading all todos from persistent storage
#[provider(stale_time = "5s", cache_expiration = "20s")]
pub async fn load_todos() -> Result<Vec<Todo>, TodoError> {
    load_todos_from_file_async().await
}

/// Helper to write todos to file asynchronously
async fn save_todos_to_file_async(todos: &[Todo]) -> Result<(), TodoError> {
    let data = serde_json::to_string_pretty(todos).map_err(|e| TodoError::Json(Arc::new(e)))?;
    fs::write(TODO_FILE, data)
        .await
        .map_err(|e| TodoError::Io(Arc::new(e)))?;
    Ok(())
}

/// Mutation: Add a new todo
#[mutation(invalidates = [load_todos])]
pub async fn add_todo(title: String) -> Result<Vec<Todo>, TodoError> {
    sleep(Duration::from_secs(1)).await; // Artificial delay for UX
    let mut todos = load_todos_from_file_async().await?;
    let id = todos.iter().map(|t| t.id).max().unwrap_or(0) + 1;
    let todo = Todo {
        id,
        title,
        completed: false,
    };
    todos.push(todo);
    save_todos_to_file_async(&todos).await?;
    Ok(todos)
}

/// Mutation: Toggle a todo's completed status
#[mutation(invalidates = [load_todos])]
pub async fn toggle_todo(id: u64) -> Result<Vec<Todo>, TodoError> {
    sleep(Duration::from_secs(1)).await; // Artificial delay for UX
    let mut todos = load_todos_from_file_async().await?;
    if let Some(todo) = todos.iter_mut().find(|t| t.id == id) {
        todo.completed = !todo.completed;
        save_todos_to_file_async(&todos).await?;
        Ok(todos)
    } else {
        Err(TodoError::NotFound)
    }
}

/// Mutation: Update a todo's title
#[mutation(invalidates = [load_todos])]
pub async fn update_todo(id: u64, new_title: String) -> Result<Vec<Todo>, TodoError> {
    sleep(Duration::from_secs(1)).await; // Artificial delay for UX
    let mut todos = load_todos_from_file_async().await?;
    if let Some(todo) = todos.iter_mut().find(|t| t.id == id) {
        todo.title = new_title;
        save_todos_to_file_async(&todos).await?;
        Ok(todos)
    } else {
        Err(TodoError::NotFound)
    }
}

/// Mutation: Delete a todo
#[mutation(invalidates = [load_todos])]
pub async fn delete_todo(id: u64) -> Result<Vec<Todo>, TodoError> {
    sleep(Duration::from_secs(1)).await; // Artificial delay for UX
    let mut todos = load_todos_from_file_async().await?;
    let len_before = todos.len();
    todos.retain(|t| t.id != id);
    if todos.len() == len_before {
        return Err(TodoError::NotFound);
    }
    save_todos_to_file_async(&todos).await?;
    Ok(todos)
}

/// Component: Input for adding a new todo
#[component]
pub fn TodoInput() -> Element {
    let mut input = use_signal(String::new);
    let (mutation_state, add) = use_mutation(add_todo());
    let cache = use_provider_cache();
    let mut temp_id_counter = use_signal(|| 0u64);

    let mut on_submit = {
        let cache = cache.clone();
        move |_| {
            let title = input.read().trim().to_string();
            if !title.is_empty() {
                // Optimistic update
                let current_id = *temp_id_counter.read();
                let temp_id = u64::MAX - current_id;
                temp_id_counter.set(current_id + 1);

                let optimistic_todo = Todo {
                    id: temp_id,
                    title: title.clone(),
                    completed: false,
                };

                let cache_key = provider_cache_key_simple(load_todos());

                // Get current todos from cache, or an empty vec if not present/error
                let mut current_todos = cache
                    .get::<Result<Vec<Todo>, TodoError>>(&cache_key)
                    .and_then(|res| res.ok())
                    .unwrap_or_default();

                // Add optimistic todo and update cache
                current_todos.push(optimistic_todo);
                cache.set(cache_key.clone(), Ok::<_, TodoError>(current_todos));

                // Trigger mutation
                add(title.clone());
                input.set(String::new());
            }
        }
    };

    let on_keydown = {
        let mut on_submit = on_submit.clone();
        let mut on_submit = on_submit.clone();
        move |e: Event<KeyboardData>| {
            if e.key() == Key::Enter {
                on_submit(());
            }
        }
    };

    // Effect for rollback on error
    use_effect({
        let cache = cache.clone();
        move || {
            if let MutationState::Error(_) = &*mutation_state.read() {
                let cache_key = provider_cache_key_simple(load_todos());
                if let Some(Ok(mut todos)) = cache.get::<Result<Vec<Todo>, TodoError>>(&cache_key) {
                    // Remove the last optimistic todo
                    let last_temp_id = u64::MAX - (*temp_id_counter.read() - 1);
                    todos.retain(|t| t.id != last_temp_id);
                    cache.set(cache_key, Ok::<_, TodoError>(todos));
                }
            }
        }
    });

    rsx! {
        form {
            class: "flex gap-2 mb-4",
            onsubmit: move |e| e.stop_propagation(),
            input {
                r#type: "text",
                value: "{input}",
                oninput: move |e| input.set(e.value().to_string()),
                onkeydown: on_keydown,
                placeholder: "What needs to be done?",
                autofocus: true,
                class: "flex-1 px-3 py-2 border border-gray-300 rounded focus:outline-none focus:ring-2 focus:ring-blue-400 bg-white text-gray-900 shadow-sm transition-all"
            }
            button {
                onclick: move |_| on_submit(()),
                class: "px-4 py-2 bg-blue-600 text-white font-semibold rounded shadow hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-400 transition-all",
                "Add"
            }
            match &*mutation_state.read() {
                MutationState::Loading => rsx!(span { class: "ml-2 text-blue-500 animate-pulse", "Adding..." }),
                MutationState::Error(err) => rsx!(span { class: "ml-2 text-red-500", "{err}" }),
                _ => rsx!(span {}),
            }
        }
    }
}

/// Component: A single todo item with edit, toggle, and delete functionality
#[component]
pub fn TodoItem(todo: Todo) -> Element {
    let mut editing = use_signal(|| false);
    let mut edit_text = use_signal(|| todo.title.clone());
    let mut optimistic_title = use_signal(|| todo.title.clone());
    let mut error_msg = use_signal(|| None as Option<String>);
    let mut optimistic_completed = use_signal(|| todo.completed);

    let (toggle_state, toggle) = use_mutation(toggle_todo());
    let (delete_state, delete) = use_mutation(delete_todo());
    let (update_state, update) = use_mutation(update_todo());

    let todo_id = todo.id;
    let todo_title = todo.title.clone();

    let on_toggle = {
        let toggle = toggle.clone();
        move |_| {
            optimistic_completed.toggle(); // Optimistically toggle
            error_msg.set(None);
            toggle(todo_id);
        }
    };

    // Listen for toggle mutation result to revert optimistic completed if error or update if success
    use_effect(move || {
        match &*toggle_state.read() {
            MutationState::Error(err) => {
                // Revert to original completed state and show error
                optimistic_completed.set(todo.completed);
                error_msg.set(Some(err.to_string()));
            }
            MutationState::Success(_) => {
                optimistic_completed.set(todo.completed);
            }
            _ => {}
        }
    });

    let on_delete = move |_| delete(todo_id);
    let on_edit = {
        let todo_title = todo_title.clone();
        move |_| {
            editing.set(true);
            edit_text.set(todo_title.clone());
            error_msg.set(None);
        }
    };
    let on_edit_input = move |e: Event<FormData>| {
        edit_text.set(e.value());
    };
    let mut on_edit_submit = {
        let update = update.clone();

        let todo_title = todo_title.clone();
        move |_| {
            let new_title = edit_text.read().trim().to_string();
            if !new_title.is_empty() && new_title != todo_title {
                optimistic_title.set(new_title.clone()); // Optimistically update UI
                error_msg.set(None);
                update((todo_id, new_title.clone()));
            }
            editing.set(false);
        }
    };
    let on_edit_keydown = {
        let mut on_edit_submit = on_edit_submit.clone();
        move |e: Event<KeyboardData>| {
            if e.key() == Key::Enter {
                on_edit_submit(());
            }
        }
    };

    // Listen for update mutation result to revert optimistic title if error
    use_effect(move || {
        if let MutationState::Error(err) = &*update_state.read() {
            // Revert to original title and show error
            optimistic_title.set(todo_title.clone());
            error_msg.set(Some(err.to_string()));
        }
    });

    // Determine which mutation is loading and set message
    let (is_mutating, mutating_msg) = if matches!(*toggle_state.read(), MutationState::Loading) {
        (true, "Toggling...")
    } else if matches!(*update_state.read(), MutationState::Loading) {
        (true, "Updating...")
    } else if matches!(*delete_state.read(), MutationState::Loading) {
        (true, "Deleting...")
    } else {
        (false, "")
    };

    rsx! {
        li { class: "flex items-center gap-3 py-2 px-2 rounded hover:bg-gray-50 group transition-all relative",
            if *editing.read() {
                div { class: "flex-1 flex gap-2 items-center",
                    input {
                        value: "{edit_text}",
                        oninput: on_edit_input,
                        onkeydown: on_edit_keydown,
                        autofocus: true,
                        class: "flex-1 px-2 py-1 border border-gray-300 rounded focus:outline-none focus:ring-2 focus:ring-blue-400 bg-white text-gray-900 shadow-sm"
                    }
                    button { onclick: move |_| on_edit_submit(()), class: "px-3 py-1 bg-green-600 text-white rounded hover:bg-green-700 transition-all", "Save" }
                }
            } else {
                input { r#type: "checkbox", checked: *optimistic_completed.read(), onclick: on_toggle, class: "accent-blue-600 w-5 h-5" }
                span {
                    onclick: on_edit,
                    class: "flex-1 cursor-pointer select-text text-lg text-gray-900 group-hover:text-blue-700 transition-all",
                    style: if *optimistic_completed.read() { "text-decoration: line-through; color: #888;" } else { "" },
                    "{optimistic_title.read()}"
                }
                button { onclick: on_delete, class: "ml-2 px-2 py-1 text-sm bg-red-500 text-white rounded hover:bg-red-600 transition-all opacity-80 group-hover:opacity-100", "Delete" }
                if is_mutating {
                    span { class: "flex items-center ml-2 gap-1",
                        div { class: "w-5 h-5 border-2 border-blue-400 border-t-transparent rounded-full animate-spin" }
                        span { class: "text-blue-600 text-sm font-medium", "{mutating_msg}" }
                    }
                }
                if let Some(err) = &*error_msg.read() {
                    span { class: "ml-2 text-red-500 text-sm", "{err}" }
                }
            }
        }
    }
}

/// Component: List of todos with filter bar
#[component]
pub fn TodoList(filter: Filter) -> Element {
    let todos = use_provider(load_todos(), ());

    let filtered_todos = match &*todos.read() {
        ProviderState::Success(todos) => {
            let filtered: Vec<Todo> = match filter {
                Filter::All => todos.clone(),
                Filter::Active => todos.iter().filter(|t| !t.completed).cloned().collect(),
                Filter::Completed => todos.iter().filter(|t| t.completed).cloned().collect(),
            };
            Some(filtered)
        }
        _ => None,
    };

    rsx! {
        div { class: "w-full",
            // Filter bar
            div { class: "flex gap-2 mb-4 justify-center",
                FilterButton { label: "All", filter: Filter::All }
                FilterButton { label: "Active", filter: Filter::Active }
                FilterButton { label: "Completed", filter: Filter::Completed }
            }
            // Todo list
            ul {
                class: "divide-y divide-gray-200",
                match &*todos.read() {
                    ProviderState::Loading { .. } => rsx!(li { class: "text-blue-500", "Loading todos..." }),
                    ProviderState::Error(err) => rsx!(li { class: "text-red-500", "Error: {err}" }),
                    ProviderState::Success(_) => {
                        if let Some(list) = filtered_todos {
                            if list.is_empty() {
                                rsx!(li { class: "text-gray-400 italic", "No todos found." })
                            } else {
                                rsx! {
                                    for todo in list {
                                        TodoItem { todo: todo.clone() }
                                    }
                                }
                            }
                        } else {
                            rsx!(li { class: "text-gray-400 italic", "No todos found." })
                        }
                    }
                }
            }
        }
    }
}

/// Filter button component
#[component]
fn FilterButton(label: &'static str, filter: Filter) -> Element {
    let mut filter_signal = use_context::<Signal<Filter>>();
    let is_selected = filter == *filter_signal.read();
    rsx! {
        button {
            onclick: move |_| filter_signal.set(filter),
            class: if is_selected {
                "px-3 py-1 font-bold rounded bg-blue-600 text-white shadow border border-blue-700 hover:bg-blue-700 transition-all"
            } else {
                "px-3 py-1 font-semibold rounded bg-gray-200 text-gray-700 hover:bg-blue-100 border border-gray-300 transition-all"
            },
            {label}
        }
    }
}

/// App component: manages filter state and composes input and list
#[component]
pub fn App() -> Element {
    let filter = use_signal(|| Filter::All);
    provide_context(filter);

    rsx! {
        script { src: "https://cdn.jsdelivr.net/npm/@tailwindcss/browser@4", defer: true }
        div { class: "min-h-screen bg-gradient-to-br from-blue-50 to-white flex items-center justify-center p-4",
            div { class: "todo-app w-full max-w-lg bg-white rounded-2xl shadow-xl border border-gray-200 p-8",
                h1 { class: "text-3xl font-bold text-center mb-6 text-blue-700 tracking-tight", "Todo App" }
                TodoInput {}
                TodoList { filter: *filter.read() }
            }
        }
    }
}

fn main() {
    dioxus_provider::global::init_global_providers();
    dioxus::launch(App);
}
