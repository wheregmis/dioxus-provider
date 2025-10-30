use dioxus::prelude::*;
use dioxus_provider::prelude::*;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::Duration;

static COUNTER: AtomicI32 = AtomicI32::new(0);

#[provider]
async fn get_counter() -> Result<i32, String> {
    #[cfg(not(target_family = "wasm"))]
    tokio::time::sleep(Duration::from_millis(100)).await;
    #[cfg(target_family = "wasm")]
    wasmtimer::tokio::sleep(Duration::from_millis(100)).await;
    Ok(COUNTER.load(Ordering::SeqCst))
}

#[mutation(invalidates = [get_counter])]
async fn increment_counter() -> Result<i32, String> {
    #[cfg(not(target_family = "wasm"))]
    tokio::time::sleep(Duration::from_millis(500)).await;
    #[cfg(target_family = "wasm")]
    wasmtimer::tokio::sleep(Duration::from_millis(500)).await;
    let val = COUNTER.fetch_add(1, Ordering::SeqCst) + 1;
    Ok(val)
}

#[component]
fn CounterApp() -> Element {
    let counter = use_provider(get_counter(), ());
    let (state, increment) = use_mutation(increment_counter());
    let invalidate = use_invalidate_provider(get_counter(), ());

    rsx! {
        div {
            class: "min-h-screen flex flex-col items-center justify-center bg-gray-100",
            h1 { class: "text-2xl font-bold mb-4", "Counter App (with Provider Invalidation)" },
            button {
                class: "px-6 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition mb-4",
                onclick: move |_| increment(()),
                "Increment (with mutation)"
            },
            button {
                class: "px-6 py-2 bg-gray-400 text-white rounded hover:bg-gray-500 transition mb-4",
                onclick: move |_| invalidate(),
                "Invalidate Provider"
            },
            h2 { class: "text-lg font-semibold mt-4", "Provider State:" },
            match &*counter.read() {
                State::Loading { .. } => rsx! { p { "Loading counter..." } },
                State::Success(val) => rsx! { p { "Counter (from provider): {val}" } },
                State::Error(err) => rsx! { p { "Error: {err}" } },
            },
            h2 { class: "text-lg font-semibold mt-4", "Mutation State:" },
            match &*state.read() {
                MutationState::Idle => rsx! { p { "Idle" } },
                MutationState::Loading => rsx! { p { "Incrementing..." } },
                MutationState::Success(val) => rsx! { p { "Mutation result: {val}" } },
                MutationState::Error(err) => rsx! { p { "Error: {err}" } },
            }
        }
    }
}

fn main() {
    COUNTER.store(0, Ordering::SeqCst);
    let _ = dioxus_provider::init();
    dioxus::launch(CounterApp);
}
