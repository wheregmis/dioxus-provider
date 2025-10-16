//! Comprehensive Todo App Example using dioxus-provider with optimistic mutations

use std::sync::Arc;

use dioxus::prelude::Key;
use dioxus::prelude::*;
use dioxus_provider::prelude::*;
use serde::{Deserialize, Serialize};
use tokio::{
    fs,
    time::{Duration, sleep},
};

/// Represents a single todo item
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Todo {
    pub id: u64,
    pub title: String,
    pub completed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TodoUpdate {
    pub id: u64,
    pub title: String,
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

fn next_todo_id(todos: &[Todo]) -> u64 {
    todos.iter().map(|t| t.id).max().unwrap_or(0) + 1
}

/// Load all todos from the persistent JSON file
async fn load_todos_from_file_async() -> Result<Vec<Todo>, TodoError> {
    match fs::read_to_string(TODO_FILE).await {
        Ok(data) => {
            let todos = serde_json::from_str(&data).map_err(|e| TodoError::Json(Arc::new(e)))?;
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
#[mutation(
    invalidates = [load_todos],
    optimistic = |todos: &mut Vec<Todo>, title: &String| {
        let id = next_todo_id(todos);
        todos.push(Todo {
            id,
            title: title.clone(),
            completed: false,
        });
    }
)]
pub async fn add_todo(_title: String, todos: Vec<Todo>) -> Result<Vec<Todo>, TodoError> {
    sleep(Duration::from_millis(400)).await;
    save_todos_to_file_async(&todos).await?;
    Ok(todos)
}

/// Mutation: Toggle a todo's completed status
#[mutation(
    invalidates = [load_todos],
    optimistic = |todos: &mut Vec<Todo>, id: &u64| {
        if let Some(todo) = todos.iter_mut().find(|t| t.id == *id) {
            todo.completed = !todo.completed;
        }
    }
)]
pub async fn toggle_todo(_id: u64, todos: Vec<Todo>) -> Result<Vec<Todo>, TodoError> {
    sleep(Duration::from_millis(250)).await;
    save_todos_to_file_async(&todos).await?;
    Ok(todos)
}

/// Mutation: Update a todo's title
#[mutation(
    invalidates = [load_todos],
    optimistic = |todos: &mut Vec<Todo>, update: &TodoUpdate| {
        if let Some(todo) = todos.iter_mut().find(|t| t.id == update.id) {
            todo.title = update.title.clone();
        }
    }
)]
pub async fn update_todo(_payload: TodoUpdate, todos: Vec<Todo>) -> Result<Vec<Todo>, TodoError> {
    sleep(Duration::from_millis(300)).await;
    save_todos_to_file_async(&todos).await?;
    Ok(todos)
}

/// Mutation: Delete a todo
#[mutation(
    invalidates = [load_todos],
    optimistic = |todos: &mut Vec<Todo>, id: &u64| {
        todos.retain(|t| t.id != *id);
    }
)]
pub async fn delete_todo(_id: u64, todos: Vec<Todo>) -> Result<Vec<Todo>, TodoError> {
    sleep(Duration::from_millis(200)).await;
    save_todos_to_file_async(&todos).await?;
    Ok(todos)
}

/// Component: Input for adding a new todo
#[component]
pub fn TodoInput() -> Element {
    let mut input = use_signal(String::new);
    let (_, add) = use_optimistic_mutation(add_todo());

    let on_keydown = {
        let add = add.clone();

        move |e: Event<KeyboardData>| {
            if e.key() == Key::Enter {
                let title = input.read().trim().to_string();
                if title.is_empty() {
                    return;
                }
                add(title);
                input.set(String::new());
            }
        }
    };

    rsx! {
        form {
            class: "flex gap-2 mb-4",
            onsubmit: {
                let add = add.clone();
                move |e| {
                    e.prevent_default();
                    let title = input.read().trim().to_string();
                    if title.is_empty() {
                        return;
                    }
                    add(title);
                    input.set(String::new());
                }
            },
            input {
                r#type: "text",
                value: "{input}",
                oninput: move |e| input.set(e.value().to_string()),
                onkeydown: on_keydown,
                placeholder: "What needs to be done?",
                autofocus: true,
                class: "flex-1 px-3 py-2 border border-gray-300 rounded focus:outline-none focus:ring-2 focus:ring-blue-400 bg-white text-gray-900 shadow-sm transition-all",
            }
            button {
                class: "px-4 py-2 bg-blue-600 text-white font-semibold rounded shadow hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-400 transition-all",
                onclick: {
                    let add = add.clone();
                    move |_| {
                        let title = input.read().trim().to_string();
                        if title.is_empty() {
                            return;
                        }
                        add(title);
                        input.set(String::new());
                    }
                },
                "Add"
            }
        }
    }
}

/// Component: A single todo item with edit, toggle, and delete functionality
#[component]
pub fn TodoItem(todo: Todo) -> Element {
    let mut editing = use_signal(|| false);
    let mut edit_text = use_signal(|| todo.title.clone());
    let (_, toggle) = use_optimistic_mutation(toggle_todo());
    let (_, delete) = use_optimistic_mutation(delete_todo());
    let (_, update) = use_optimistic_mutation(update_todo());

    let todo_id = todo.id;
    let todo_title = todo.title.clone();

    let on_toggle = {
        let toggle = toggle.clone();
        move |_| toggle(todo_id)
    };

    let on_delete = move |_| delete(todo_id);

    let on_edit = {
        let todo_title = todo_title.clone();

        move |_| {
            editing.set(true);
            edit_text.set(todo_title.clone());
        }
    };

    let on_edit_input = {
        move |e: Event<FormData>| {
            edit_text.set(e.value());
        }
    };

    let mut submit_edit = {
        let update = update.clone();
        move |_| {
            let new_title = edit_text.read().trim().to_string();
            if !new_title.is_empty() && new_title != todo_title {
                update(TodoUpdate {
                    id: todo_id,
                    title: new_title,
                });
            }
            editing.set(false);
        }
    };

    let on_edit_keydown = {
        let mut submit_edit = submit_edit.clone();
        move |e: Event<KeyboardData>| {
            if e.key() == Key::Enter {
                submit_edit(());
            }
        }
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
                        class: "flex-1 px-2 py-1 border border-gray-300 rounded focus:outline-none focus:ring-2 focus:ring-blue-400 bg-white text-gray-900 shadow-sm",
                    }
                    button {
                        onclick: move |_| submit_edit(()),
                        class: "px-3 py-1 bg-green-600 text-white rounded hover:bg-green-700 transition-all",
                        "Save"
                    }
                }
            } else {
                input {
                    r#type: "checkbox",
                    checked: todo.completed,
                    onclick: on_toggle,
                    class: "accent-blue-600 w-5 h-5",
                }
                span {
                    onclick: on_edit,
                    class: "flex-1 cursor-pointer select-text text-lg text-gray-900 group-hover:text-blue-700 transition-all",
                    style: if todo.completed { "text-decoration: line-through; color: #888;" } else { "text-decoration: none; color: inherit;" },
                    "{todo.title}"
                }
                button {
                    onclick: on_delete,
                    class: "ml-2 px-2 py-1 text-sm bg-red-500 text-white rounded hover:bg-red-600 transition-all opacity-80 group-hover:opacity-100",
                    "Delete"
                }
            }
        }
    }
}

/// Component: List of todos filtered by the active filter value
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
            match &*todos.read() {
                ProviderState::Loading { .. } => rsx! {
                    div { class: "text-center text-gray-500", "Loading todos..." }
                },
                ProviderState::Error(err) => rsx! {
                    div { class: "text-center text-red-500", "Failed to load todos: {err}" }
                },
                ProviderState::Success(_) => rsx! {
                    if let Some(filtered) = filtered_todos {
                        if filtered.is_empty() {
                            div { class: "text-center text-gray-500", "No todos found" }
                        } else {
                            ul { class: "space-y-2",
                                for todo in filtered {
                                    TodoItem { todo }
                                }
                            }
                        }
                    }
                },
            }
        }
    }
}

/// Filter bar component allowing the user to change the active filter
#[component]
pub fn FilterBar(filter: Signal<Filter>) -> Element {
    let render_button = |label: &'static str, target: Filter| {
        let is_active = *filter.read() == target;
        rsx! {
            button {
                onclick: move |_| filter.set(target),
                class: {
                    let base = "px-3 py-1 rounded-full border transition-all";
                    if is_active {
                        format!("{base} bg-blue-600 text-white border-blue-600")
                    } else {
                        format!(
                            "{base} bg-white text-gray-700 border-gray-300 hover:border-blue-400",
                        )
                    }
                },
                "{label}"
            }
        }
    };

    rsx! {
        div { class: "flex gap-2 mb-4 justify-center",
            {render_button("All", Filter::All)}
            {render_button("Active", Filter::Active)}
            {render_button("Completed", Filter::Completed)}
        }
    }
}

/// Root app component
#[component]
pub fn TodoApp() -> Element {
    let filter = use_signal(|| Filter::All);

    rsx! {
        div { class: "min-h-screen bg-gray-100 flex flex-col items-center py-10 px-4",
            div { class: "w-full max-w-3xl bg-white shadow-lg rounded-xl p-6 space-y-6",
                header { class: "text-center space-y-2",
                    h1 { class: "text-4xl font-bold text-gray-900", "Todo App" }
                    p { class: "text-gray-500", "Demonstrates providers + optimistic mutations" }
                }

                TodoInput {}
                FilterBar { filter }
                TodoList { filter: *filter.read() }
            }
        }
    }
}

fn main() {
    let _ = dioxus_provider::init();
    dioxus::launch(TodoApp);
}
