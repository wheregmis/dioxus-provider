use dioxus::prelude::*;
use dioxus_core::NoOpMutations;
use dioxus_provider::global;
use dioxus_provider::hooks::Provider;
use dioxus_provider::prelude::{State, use_provider};
use futures::FutureExt;
use std::future::Future;
use std::rc::Rc;
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};
use std::time::Duration;
use tokio::{task::yield_now, time::sleep};

#[derive(Clone)]
struct CountingProvider {
    calls: Arc<AtomicU32>,
}

impl CountingProvider {
    fn new() -> (Self, Arc<AtomicU32>) {
        let calls = Arc::new(AtomicU32::new(0));
        (
            Self {
                calls: calls.clone(),
            },
            calls,
        )
    }
}

impl PartialEq for CountingProvider {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Provider<()> for CountingProvider {
    type Output = u32;
    type Error = ();

    fn run(
        &self,
        _param: (),
    ) -> impl std::future::Future<Output = Result<Self::Output, Self::Error>> {
        let calls = self.calls.clone();
        async move {
            let value = calls.fetch_add(1, Ordering::SeqCst) + 1;
            sleep(Duration::from_millis(10)).await;
            Ok(value)
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct ConsumerProps {
    provider: CountingProvider,
    recorder: Rc<std::cell::RefCell<Vec<State<u32, ()>>>>,
}

#[allow(non_snake_case)]
fn Consumer(props: ConsumerProps) -> Element {
    let state = use_provider(props.provider.clone(), ());
    let record = props.recorder.clone();
    use_effect(move || {
        record.borrow_mut().push(state.read().clone());
    });
    rsx!(div {})
}

#[derive(Props, Clone, PartialEq)]
struct DualConsumerProps {
    provider: CountingProvider,
    recorder_a: Rc<std::cell::RefCell<Vec<State<u32, ()>>>>,
    recorder_b: Rc<std::cell::RefCell<Vec<State<u32, ()>>>>,
}

#[allow(non_snake_case)]
fn DualConsumer(props: DualConsumerProps) -> Element {
    rsx! {
        Consumer {
            provider: props.provider.clone(),
            recorder: props.recorder_a.clone()
        }
        Consumer {
            provider: props.provider.clone(),
            recorder: props.recorder_b.clone()
        }
    }
}

fn block_on_test(fut: impl Future<Output = ()>) {
    tokio::runtime::Runtime::new()
        .expect("tokio runtime")
        .block_on(fut);
}

#[test]
fn dedupes_parallel_consumers() {
    block_on_test(async {
        let _ = global::init();
        let (provider, call_count) = CountingProvider::new();
        let recorder_a = Rc::new(std::cell::RefCell::new(Vec::new()));
        let recorder_b = Rc::new(std::cell::RefCell::new(Vec::new()));

        let mut vdom = VirtualDom::new_with_props(
            DualConsumer,
            DualConsumerProps {
                provider,
                recorder_a: recorder_a.clone(),
                recorder_b: recorder_b.clone(),
            },
        );
        vdom.rebuild_in_place();
        let mut mutations = NoOpMutations;
        for _ in 0..3 {
            while vdom.wait_for_work().now_or_never().is_some() {
                vdom.render_immediate(&mut mutations);
            }
            yield_now().await;
        }

        assert_eq!(
            call_count.load(Ordering::SeqCst),
            1,
            "both consumers should share a single provider run"
        );
    });
}
