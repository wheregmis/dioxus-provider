//! Request orchestration helpers for use_provider.

use dioxus::prelude::*;

use crate::{
    cache::ProviderCache, hooks::Provider, refresh::RefreshRegistry, runtime::ProviderRuntime,
    state::State, types::ProviderParamBounds,
};

/// State handle abstraction so runtime logic can be tested without real Dioxus signals.
pub trait RuntimeStateHandle<T, E>: Clone {
    fn set_state(&mut self, new_state: State<T, E>);
    fn set_state_if_changed(&mut self, new_state: State<T, E>) {
        self.set_state(new_state);
    }
    fn is_loading(&self) -> bool;
}

impl<T: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> RuntimeStateHandle<T, E>
    for Signal<State<T, E>>
{
    fn set_state(&mut self, new_state: State<T, E>) {
        self.set(new_state);
    }

    fn set_state_if_changed(&mut self, new_state: State<T, E>) {
        let should_update = {
            let current = self.read();
            match (&*current, &new_state) {
                (State::Success(existing), State::Success(next)) => existing != next,
                (State::Error(existing), State::Error(next)) => existing != next,
                (State::Loading { .. }, State::Loading { .. }) => true,
                (State::Success(_), State::Error(_))
                | (State::Success(_), State::Loading { .. })
                | (State::Error(_), State::Success(_))
                | (State::Error(_), State::Loading { .. })
                | (State::Loading { .. }, State::Success(_))
                | (State::Loading { .. }, State::Error(_)) => true,
            }
        };

        if should_update {
            self.set(new_state);
        }
    }

    fn is_loading(&self) -> bool {
        self.read().is_loading()
    }
}

/// Cache miss orchestration that handles pending-request dedupe, invalidation SWR,
/// and the primary async execution.
pub fn handle_cache_miss<P, Param, Handle>(
    runtime: &ProviderRuntime,
    provider: P,
    param: Param,
    cache: ProviderCache,
    refresh_registry: RefreshRegistry,
    cache_key: String,
    state: Handle,
) where
    P: Provider<Param> + Send + Clone,
    Param: ProviderParamBounds,
    Handle: RuntimeStateHandle<P::Output, P::Error> + 'static,
{
    let is_new_request = runtime.mark_request_pending(&cache_key);

    if !is_new_request {
        #[cfg(feature = "tracing")]
        {
            let pending_count = runtime.pending_request_count(&cache_key);
            if pending_count == 1
                || pending_count == 2
                || pending_count == 4
                || pending_count == 8
                || pending_count == 16
                || pending_count == 100
                || pending_count == 200
                || pending_count == 500
                || pending_count % 1000 == 0
            {
                crate::debug_log!(
                    "🔄 [REQUEST-DEDUP] Request already pending for key: {} ({} components waiting)",
                    cache_key,
                    pending_count
                );
            }
        }

        if !state.is_loading() {
            let mut loading_handle = state.clone();
            loading_handle.set_state(State::Loading {
                task: dioxus::prelude::spawn(async {}),
            });
        }
        return;
    }

    crate::debug_log!(
        "🆕 [REQUEST-DEDUP] Starting new request for key: {}",
        cache_key
    );

    let is_invalidation_refresh = refresh_registry.get_refresh_count(&cache_key) > 0;

    if is_invalidation_refresh {
        crate::debug_log!(
            "🔄 [INVALIDATION] Cache miss due to invalidation for: {}, using SWR behavior",
            cache_key
        );

        let cache_clone = cache.clone();
        let cache_key_clone = cache_key.clone();
        let provider = provider.clone();
        let param = param.clone();
        let refresh_registry_clone = refresh_registry.clone();
        let runtime_clone = runtime.clone();

        dioxus::prelude::spawn(async move {
            let result = provider.run(param).await;
            let updated = cache_clone.set(cache_key_clone.clone(), result.clone());
            if updated {
                refresh_registry_clone.trigger_refresh(&cache_key_clone);
                crate::debug_log!(
                    "✅ [INVALIDATION] Background revalidation completed for: {}",
                    cache_key_clone
                );
            }
            runtime_clone.mark_request_complete(&cache_key_clone);
        });

        return;
    }

    let cache_clone = cache.clone();
    let cache_key_clone = cache_key.clone();
    let provider_clone = provider.clone();
    let param_clone = param.clone();
    let refresh_registry_clone = refresh_registry.clone();
    let runtime_clone = runtime.clone();
    let mut state_for_async = state.clone();

    let task = dioxus::prelude::spawn(async move {
        let result = provider_clone.run(param_clone).await;
        let updated = cache_clone.set(cache_key_clone.clone(), result.clone());
        crate::debug_log!(
            "📊 [CACHE-STORE] Attempted to store new data for: {} (updated: {})",
            cache_key_clone,
            updated
        );
        if updated {
            match result {
                Ok(data) => {
                    state_for_async.set_state(State::Success(data));
                }
                Err(error) => {
                    state_for_async.set_state(State::Error(error));
                }
            }
        }
        runtime_clone.mark_request_complete(&cache_key_clone);
        refresh_registry_clone.trigger_refresh(&cache_key_clone);
    });
    let mut state_for_loading = state;
    state_for_loading.set_state(State::Loading { task });
}

/// Resolve the current state from cache or kick off the request pipeline.
pub fn resolve_cache_or_fetch<P, Param, Handle>(
    runtime: &ProviderRuntime,
    provider: P,
    param: Param,
    cache: ProviderCache,
    refresh_registry: RefreshRegistry,
    cache_key: String,
    state: Handle,
) where
    P: Provider<Param> + Send + Clone,
    Param: ProviderParamBounds,
    Handle: RuntimeStateHandle<P::Output, P::Error> + 'static,
{
    if let Some(cached_result) = cache.get::<Result<P::Output, P::Error>>(&cache_key) {
        let mut state_handle = state;
        match cached_result {
            Ok(data) => state_handle.set_state_if_changed(State::Success(data)),
            Err(error) => state_handle.set_state_if_changed(State::Error(error)),
        }
        return;
    }

    handle_cache_miss(
        runtime,
        provider,
        param,
        cache,
        refresh_registry,
        cache_key,
        state,
    );
}

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::*;
    use crate::runtime::{ProviderRuntime, ProviderRuntimeConfig};
    use dioxus::prelude::{Element, ScopeId, VirtualDom, rsx};
    use dioxus_core::NoOpMutations;
    use futures::FutureExt;
    use std::{
        future::Future,
        sync::{
            Arc,
            atomic::{AtomicBool, AtomicU32, Ordering},
        },
        time::Duration,
    };
    use tokio::time::sleep;

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

    #[derive(Clone, Default)]
    struct TestStateHandle {
        is_loading: Arc<AtomicBool>,
        saw_success: Arc<AtomicBool>,
        loading_after_success: Arc<AtomicBool>,
    }

    impl TestStateHandle {
        fn entered_loading_after_success(&self) -> bool {
            self.loading_after_success.load(Ordering::SeqCst)
        }

        fn is_loading_now(&self) -> bool {
            self.is_loading.load(Ordering::SeqCst)
        }
    }

    impl<T, E> RuntimeStateHandle<T, E> for TestStateHandle {
        fn set_state(&mut self, state: State<T, E>) {
            match state {
                State::Loading { .. } => {
                    if self.saw_success.load(Ordering::SeqCst) {
                        self.loading_after_success.store(true, Ordering::SeqCst);
                    }
                    self.is_loading.store(true, Ordering::SeqCst);
                }
                State::Success(_) => {
                    self.saw_success.store(true, Ordering::SeqCst);
                    self.is_loading.store(false, Ordering::SeqCst);
                }
                State::Error(_) => {
                    self.is_loading.store(false, Ordering::SeqCst);
                }
            }
        }

        fn is_loading(&self) -> bool {
            self.is_loading.load(Ordering::SeqCst)
        }
    }

    struct DioxusRuntimeHarness {
        dom: VirtualDom,
    }

    impl DioxusRuntimeHarness {
        fn new() -> Self {
            fn idle() -> Element {
                rsx!(div {})
            }

            let mut dom = VirtualDom::new(idle);
            dom.rebuild_in_place();
            Self { dom }
        }

        fn run<R>(&self, f: impl FnOnce() -> R) -> R {
            self.dom.runtime().in_scope(ScopeId::ROOT, f)
        }

        fn pump(&mut self) {
            let mut mutations = NoOpMutations;
            while self.dom.wait_for_work().now_or_never().is_some() {
                self.dom.render_immediate(&mut mutations);
            }
        }
    }

    fn block_on<F: Future<Output = ()>>(future: F) {
        tokio::runtime::Runtime::new()
            .expect("tokio runtime")
            .block_on(future);
    }

    #[test]
    fn dedupes_inflight_requests_and_marks_waiters_loading() {
        block_on(async {
            let mut harness = DioxusRuntimeHarness::new();
            let runtime = ProviderRuntime::new(ProviderRuntimeConfig::new());
            let handles = runtime.handles();
            let (provider, calls) = CountingProvider::new();
            let cache_key = "dedupe-key".to_string();

            let first_handle = TestStateHandle::default();
            let second_handle = TestStateHandle::default();

            harness.run(|| {
                handle_cache_miss(
                    &runtime,
                    provider.clone(),
                    (),
                    handles.cache.clone(),
                    handles.refresh_registry.clone(),
                    cache_key.clone(),
                    first_handle.clone(),
                );
            });
            harness.run(|| {
                handle_cache_miss(
                    &runtime,
                    provider.clone(),
                    (),
                    handles.cache.clone(),
                    handles.refresh_registry.clone(),
                    cache_key.clone(),
                    second_handle.clone(),
                );
            });
            harness.pump();

            assert!(
                first_handle.is_loading_now() && second_handle.is_loading_now(),
                "all waiters should enter loading while deduping"
            );
            assert_eq!(
                calls.load(Ordering::SeqCst),
                1,
                "deduped requests should only trigger a single provider run"
            );

            sleep(Duration::from_millis(30)).await;
            harness.pump();
            assert_eq!(
                calls.load(Ordering::SeqCst),
                1,
                "inflight dedupe should not trigger extra runs"
            );
        });
    }

    #[test]
    fn allows_new_request_after_completion() {
        block_on(async {
            let mut harness = DioxusRuntimeHarness::new();
            let runtime = ProviderRuntime::new(ProviderRuntimeConfig::new());
            let handles = runtime.handles();
            let (provider, calls) = CountingProvider::new();
            let cache_key = "dedupe-key-2".to_string();

            let handle = TestStateHandle::default();
            harness.run(|| {
                handle_cache_miss(
                    &runtime,
                    provider.clone(),
                    (),
                    handles.cache.clone(),
                    handles.refresh_registry.clone(),
                    cache_key.clone(),
                    handle.clone(),
                );
            });
            harness.pump();
            sleep(Duration::from_millis(30)).await;
            harness.pump();
            assert_eq!(calls.load(Ordering::SeqCst), 1);

            let follow_up_handle = TestStateHandle::default();
            harness.run(|| {
                handle_cache_miss(
                    &runtime,
                    provider.clone(),
                    (),
                    handles.cache.clone(),
                    handles.refresh_registry.clone(),
                    cache_key.clone(),
                    follow_up_handle.clone(),
                );
            });
            harness.pump();
            sleep(Duration::from_millis(30)).await;
            harness.pump();

            assert_eq!(
                calls.load(Ordering::SeqCst),
                2,
                "once prior request completes the next call should start a new run"
            );
        });
    }

    #[test]
    fn swr_invalidation_runs_in_background_without_loading() {
        block_on(async {
            let mut harness = DioxusRuntimeHarness::new();
            let runtime = ProviderRuntime::new(ProviderRuntimeConfig::new());
            let handles = runtime.handles();
            let (provider, calls) = CountingProvider::new();
            let cache_key = "swr-key".to_string();

            let initial_handle = TestStateHandle::default();
            harness.run(|| {
                handle_cache_miss(
                    &runtime,
                    provider.clone(),
                    (),
                    handles.cache.clone(),
                    handles.refresh_registry.clone(),
                    cache_key.clone(),
                    initial_handle.clone(),
                );
            });
            harness.pump();

            sleep(Duration::from_millis(30)).await;
            harness.pump();
            assert_eq!(calls.load(Ordering::SeqCst), 1);

            handles.cache.invalidate(&cache_key);
            handles.refresh_registry.trigger_refresh(&cache_key);

            let swr_handle = TestStateHandle::default();
            harness.run(|| {
                handle_cache_miss(
                    &runtime,
                    provider.clone(),
                    (),
                    handles.cache.clone(),
                    handles.refresh_registry.clone(),
                    cache_key.clone(),
                    swr_handle.clone(),
                );
            });
            harness.pump();

            sleep(Duration::from_millis(30)).await;
            harness.pump();
            assert_eq!(calls.load(Ordering::SeqCst), 2);
            assert!(
                !swr_handle.entered_loading_after_success(),
                "SWR invalidation should not re-enter loading state"
            );
        });
    }
}
