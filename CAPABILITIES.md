## Capabilities Matrix

This matrix captures the behaviours showcased in the shipped examples so we can preserve them while simplifying the runtime.

| Capability | Example(s) | Status |
| --- | --- | --- |
| Lifecycle controls (interval/SWR/TTL) | `examples/comprehensive_demo.rs` (`fetch_live_metrics`, `fetch_user_dashboard`, `fetch_analytics_report`, `fetch_chat_messages`) | Stable |
| Optimistic mutations w/ invalidation | `examples/todo_app.rs:137-196` (`add_todo`, `toggle_todo`, `update_todo`, `delete_todo`) | Experimental |
| Provider composition (parallel fetch) | `examples/composable_provider_demo.rs:1-195` | Stable |
| Dependency injection (global resources) | `examples/dependency_injection_demo.rs:93-199` | Stable |

> **Note:** Optimistic mutations are still evolving and may change during cleanup; treat that surface as experimental until the new runtime lands.
