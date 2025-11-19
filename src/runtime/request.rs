//! Request orchestration helpers for use_provider.

use dioxus::prelude::*;

use crate::{
    cache::ProviderCache, hooks::Provider, refresh::RefreshRegistry, runtime::ProviderRuntime,
    state::State, types::ProviderParamBounds,
};

/// Cache miss orchestration that handles pending-request dedupe, invalidation SWR,
/// and the primary async execution.
pub fn handle_cache_miss<P, Param>(
    runtime: &ProviderRuntime,
    provider: P,
    param: Param,
    cache: ProviderCache,
    refresh_registry: RefreshRegistry,
    cache_key: String,
    mut state: Signal<State<P::Output, P::Error>>,
) where
    P: Provider<Param> + Send + Clone,
    Param: ProviderParamBounds,
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

        if !state.read().is_loading() {
            state.set(State::Loading {
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
                Ok(data) => state_for_async.set(State::Success(data)),
                Err(error) => state_for_async.set(State::Error(error)),
            }
        }
        runtime_clone.mark_request_complete(&cache_key_clone);
        refresh_registry_clone.trigger_refresh(&cache_key_clone);
    });
    state.set(State::Loading { task });
}
