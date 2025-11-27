//! Comprehensive Todo App Example using the new simplified provider API
//! with optimistic mutations

use dioxus::prelude::Key;
use dioxus::prelude::*;
use dioxus_provider::provider::{use_provider, State};
use dioxus_provider::mutation::use_mutation;
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
#[derive(Debug, Clone, PartialEq)]
pub enum TodoError {
    Io(String),
    Json(String),
    NotFound,
    Other(String),
}

impl std::fmt::Display for TodoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TodoError::Io(e) => write!(f, "IO error: {}", e),
            TodoError::Json(e) => write!(f, "JSON error: {}", e),
            TodoError::NotFound => write!(f, "Todo not found"),
            TodoError::Other(s) => write!(f, "{}", s),
        }
    }
}

const TODO_FILE: &str = "todos.json";

fn next_todo_id(todos: &[Todo]) -> u64 {
    todos.iter().map(|t| t.id).max().unwrap_or(0) + 1
}

/// Load all todos from the persistent JSON file
async fn load_todos() -> Result<Vec<Todo>, TodoError> {
    match fs::read_to_string(TODO_FILE).await {
        Ok(data) => {
            let todos = serde_json::from_str(&data).map_err(|e| TodoError::Json(e.to_string()))?;
            Ok(todos)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(vec![]),
        Err(e) => Err(TodoError::Io(e.to_string())),
    }
}

/// Statistics about todos
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TodoStats {
    pub total: usize,
    pub completed: usize,
    pub active: usize,
    pub completion_percentage: f64,
}

/// Load todo statistics
async fn load_todo_stats() -> Result<TodoStats, TodoError> {
    let todos = load_todos().await?;

    let total = todos.len();
    let completed = todos.iter().filter(|t| t.completed).count();
    let active = total - completed;
    let completion_percentage = if total > 0 {
        (completed as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    Ok(TodoStats {
        total,
        completed,
        active,
        completion_percentage,
    })
}

/// Helper to write todos to file asynchronously
async fn save_todos_to_file(todos: &[Todo]) -> Result<(), TodoError> {
    let data = serde_json::to_string_pretty(todos).map_err(|e| TodoError::Json(e.to_string()))?;
    fs::write(TODO_FILE, data)
        .await
        .map_err(|e| TodoError::Io(e.to_string()))?;
    Ok(())
}

/// Add a new todo
async fn add_todo(title: String) -> Result<Vec<Todo>, TodoError> {
    sleep(Duration::from_millis(400)).await;
    let mut todos = load_todos().await?;
    let id = next_todo_id(&todos);
    todos.push(Todo {
        id,
        title,
        completed: false,
    });
    save_todos_to_file(&todos).await?;
    Ok(todos)
}

/// Toggle a todo's completed status
async fn toggle_todo(id: u64) -> Result<Vec<Todo>, TodoError> {
    sleep(Duration::from_millis(250)).await;
    let mut todos = load_todos().await?;
    if let Some(todo) = todos.iter_mut().find(|t| t.id == id) {
        todo.completed = !todo.completed;
    }
    save_todos_to_file(&todos).await?;
    Ok(todos)
}

/// Update a todo's title
async fn update_todo(id: u64, title: String) -> Result<Vec<Todo>, TodoError> {
    sleep(Duration::from_millis(300)).await;
    let mut todos = load_todos().await?;
    if let Some(todo) = todos.iter_mut().find(|t| t.id == id) {
        todo.title = title;
    }
    save_todos_to_file(&todos).await?;
    Ok(todos)
}

/// Delete a todo
async fn delete_todo(id: u64) -> Result<Vec<Todo>, TodoError> {
    sleep(Duration::from_millis(200)).await;
    let mut todos = load_todos().await?;
    todos.retain(|t| t.id != id);
    save_todos_to_file(&todos).await?;
    Ok(todos)
}

/// Component: Input for adding a new todo
#[component]
pub fn TodoInput() -> Element {
    let mut input = use_signal(String::new);
    
    // Create mutation with invalidation
    let mut add_mutation = use_mutation(add_todo)
        .invalidates(load_todos)
        .invalidates(load_todo_stats);

    let on_keydown = move |e: Event<KeyboardData>| {
        if e.key() == Key::Enter {
            let title = input.read().trim().to_string();
            if !title.is_empty() {
                add_mutation.call(title);
                input.set(String::new());
            }
        }
    };

    rsx! {
        form {
            class: "flex gap-2 mb-4",
            onsubmit: move |e| {
                e.prevent_default();
                let title = input.read().trim().to_string();
                if !title.is_empty() {
                    add_mutation.call(title);
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
                disabled: add_mutation.pending(),
                onclick: move |_| {
                    let title = input.read().trim().to_string();
                    if !title.is_empty() {
                        add_mutation.call(title);
                        input.set(String::new());
                    }
                },
                if add_mutation.pending() { "Adding..." } else { "Add" }
            }
        }
    }
}

/// Component: A single todo item with edit, toggle, and delete functionality
#[component]
pub fn TodoItem(todo: Todo) -> Element {
    let mut editing = use_signal(|| false);
    let mut edit_text = use_signal(|| todo.title.clone());
    
    // Create mutations with invalidations
    let mut toggle_mutation = use_mutation(toggle_todo)
        .invalidates(load_todos)
        .invalidates(load_todo_stats);
    
    let mut delete_mutation = use_mutation(delete_todo)
        .invalidates(load_todos)
        .invalidates(load_todo_stats);
    
    let mut update_mutation = use_mutation(update_todo)
        .invalidates(load_todos)
        .invalidates(load_todo_stats);

    let todo_id = todo.id;
    let todo_title = todo.title.clone();

    let on_toggle = move |_| toggle_mutation.call(todo_id);
    let on_delete = move |_| delete_mutation.call(todo_id);

    let on_edit = {
        let todo_title = todo_title.clone();
        move |_| {
            editing.set(true);
            edit_text.set(todo_title.clone());
        }
    };

    let on_edit_input = move |e: Event<FormData>| {
        edit_text.set(e.value());
    };

    let on_edit_keydown = {
        let todo_title = todo_title.clone();
        move |e: Event<KeyboardData>| {
            if e.key() == Key::Enter {
                let new_title = edit_text.read().trim().to_string();
                if !new_title.is_empty() && new_title != todo_title {
                    update_mutation.call(todo_id, new_title);
                }
                editing.set(false);
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
                        onclick: move |_| {
                            let new_title = edit_text.read().trim().to_string();
                            if !new_title.is_empty() && new_title != todo_title {
                                update_mutation.call(todo_id, new_title);
                            }
                            editing.set(false);
                        },
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
                    disabled: delete_mutation.pending(),
                    class: "ml-2 px-2 py-1 text-sm bg-red-500 text-white rounded hover:bg-red-600 transition-all opacity-80 group-hover:opacity-100",
                    if delete_mutation.pending() { "..." } else { "Delete" }
                }
            }
        }
    }
}

/// Component: List of todos filtered by the active filter value
#[component]
pub fn TodoList(filter: Filter) -> Element {
    let mut todos_provider = use_provider(load_todos)
        .stale_time(Duration::from_secs(5))
        .cache_expiration(Duration::from_secs(20));

    // Fetch on mount
    use_effect(move || {
        todos_provider.call();
    });

    let filtered_todos = match todos_provider.state() {
        State::Ready => {
            if let Some(todos) = todos_provider.get_data() {
                let filtered: Vec<Todo> = match filter {
                    Filter::All => todos.clone(),
                    Filter::Active => todos.iter().filter(|t| !t.completed).cloned().collect(),
                    Filter::Completed => todos.iter().filter(|t| t.completed).cloned().collect(),
                };
                Some(filtered)
            } else {
                None
            }
        }
        _ => None,
    };

    rsx! {
        div { class: "w-full",
            match todos_provider.state() {
                State::Pending | State::Idle => rsx! {
                    div { class: "text-center text-gray-500", "Loading todos..." }
                },
                State::Error => rsx! {
                    div { class: "text-center text-red-500", 
                        "Failed to load todos: {todos_provider.error().map(|e| e.to_string()).unwrap_or_default()}" 
                    }
                },
                State::Ready => rsx! {
                    if let Some(filtered) = filtered_todos {
                        if filtered.is_empty() {
                            div { class: "text-center text-gray-500", "No todos found" }
                        } else {
                            ul { class: "space-y-2",
                                for todo in filtered {
                                    TodoItem { key: "{todo.id}", todo }
                                }
                            }
                        }
                    }
                },
                _ => rsx! { div { "Reset" } },
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

/// Component: Display todo statistics
#[component]
pub fn TodoStatsDisplay() -> Element {
    let mut stats_provider = use_provider(load_todo_stats)
        .stale_time(Duration::from_secs(5))
        .cache_expiration(Duration::from_secs(20));

    // Fetch on mount
    use_effect(move || {
        stats_provider.call();
    });

    rsx! {
        div { class: "bg-blue-50 border border-blue-200 rounded-lg p-4",
            match stats_provider.state() {
                State::Pending | State::Idle => rsx! {
                    div { class: "text-center text-blue-600", "Loading stats..." }
                },
                State::Error => rsx! {
                    div { class: "text-center text-red-500", 
                        "Failed to load stats: {stats_provider.error().map(|e| e.to_string()).unwrap_or_default()}" 
                    }
                },
                State::Ready => {
                    if let Some(stats) = stats_provider.get_data() {
                        rsx! {
                            div { class: "grid grid-cols-2 md:grid-cols-4 gap-4 text-center",
                                div { class: "space-y-1",
                                    div { class: "text-2xl font-bold text-blue-600", "{stats.total}" }
                                    div { class: "text-sm text-gray-600", "Total" }
                                }
                                div { class: "space-y-1",
                                    div { class: "text-2xl font-bold text-green-600", "{stats.completed}" }
                                    div { class: "text-sm text-gray-600", "Completed" }
                                }
                                div { class: "space-y-1",
                                    div { class: "text-2xl font-bold text-orange-600", "{stats.active}" }
                                    div { class: "text-sm text-gray-600", "Active" }
                                }
                                div { class: "space-y-1",
                                    div { class: "text-2xl font-bold text-purple-600",
                                        "{(stats.completion_percentage as u32)}%"
                                    }
                                    div { class: "text-sm text-gray-600", "Complete" }
                                }
                            }
                        }
                    } else {
                        rsx! { div { "No stats" } }
                    }
                },
                _ => rsx! { div { "Reset" } },
            }
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
                    p { class: "text-gray-500", "Using new simplified provider API with mutations" }
                }

                TodoStatsDisplay {}
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
