## Capabilities Matrix

This matrix captures the behaviours showcased in the shipped examples so we can preserve them while simplifying the runtime.

| Capability | Example(s) | Status |
| --- | --- | --- |
| Lifecycle controls (interval/SWR/TTL) | `examples/features_overview.rs` (Basic & Lifecycle tab) | Stable |
| Optimistic mutations w/ invalidation | `examples/optimistic_minimal.rs`, `examples/todo_app.rs` | Stable |
| Provider composition (parallel fetch) | `examples/features_overview.rs` (Composition tab) | Stable |
| Dependency injection (global resources) | `examples/features_overview.rs` (Parameterized & DI tab) | Stable |

> **Note:** Optimistic mutations are still evolving and may change during cleanup; treat that surface as experimental until the new runtime lands.
