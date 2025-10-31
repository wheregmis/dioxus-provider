use dioxus::prelude::*;
use dioxus_provider::platform::sleep;
use dioxus_provider::prelude::*;
use std::time::Duration;

// A provider that fetches a user's age (simulated async)
#[provider]
async fn fetch_user_age(user_id: u32) -> Result<u32, String> {
    // Simulate network delay
    sleep(Duration::from_millis(800)).await;
    if user_id == 0 {
        Err("User not found".to_string())
    } else {
        Ok(25)
    }
}

#[component]
fn UserAgeCard(user_id: u32) -> Element {
    let state = use_provider(fetch_user_age(), user_id);

    // Use combinators to transform and handle the state
    let message: State<String, String> = state
        .read()
        .clone()
        .map(|age| format!("User is {age} years old."))
        .map_err(|err| format!("Could not load age: {err}"))
        .and_then(|msg| {
            // Only show a special message if age > 21
            if msg.contains("25") {
                State::Success(format!("{msg} (Eligible for premium features!)"))
            } else {
                State::Success(msg)
            }
        });

    rsx! {
        match &message {
            State::Loading { .. } => rsx!(div { "Loading age..." }),
            State::Success(msg) => rsx!(div { "{msg}" }),
            State::Error(err) => rsx!(div { style: "color: red;", "{err}" }),
        }
    }
}

#[component]
fn App() -> Element {
    let mut user_id = use_signal(|| 1u32);

    rsx! {
        div { class: "container",
            h1 { "ProviderState Combinators Demo" }
            button {
                onclick: move |_| user_id += 1,
                "Next User"
            }
            UserAgeCard { user_id: *user_id.read() }
        }
    }
}

fn main() {
    let _ = dioxus_provider::init();
    launch(App);
}
