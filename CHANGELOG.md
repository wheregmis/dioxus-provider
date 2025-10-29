# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Optimistic mutations now accept an `optimistic = |cache, input| { .. }` closure and a
  `MutationContext` parameter, keeping optimistic and server logic in one place.
- `use_optimistic_mutation` reuses optimistic cache results on success to avoid visible
  loading flicker in UIs.
- Refreshed documentation: new concise README, updated optimistic example docs, and notes
  in the minimal/todo demos.

### Changed
- Updated to `dioxus` v0.7.0-rc.0.
- `ProviderState` gained `map`, `map_err`, and `and_then` combinators for ergonomic UI code.

## [0.0.6](https://github.com/wheregmis/dioxus-provider/compare/dioxus-provider-v0.0.5...dioxus-provider-v0.0.6) - 2025-07-12

### <!-- 3 -->Other

- Rename AsyncState to ProviderState throughout codebase
- Support for Suspense
### Breaking Change
- `AsyncState` has been renamed to `ProviderState` for clarity and consistency with the library's naming. The loading state is now `ProviderState::Loading { task: Task }`. All pattern matches must use `ProviderState::Loading { .. }` instead of `AsyncState::Loading`. This affects all provider and consumer code that matches on loading state.

## [0.0.5](https://github.com/wheregmis/dioxus-provider/compare/dioxus-provider-v0.0.4...dioxus-provider-v0.0.5) - 2025-07-08

### <!-- 3 -->Other

- Optimize cache set to avoid unnecessary updates
- Make CacheEntry cached_at field thread-safe
- Reapply "Avoid redundant cache updates for unchanged values"

## [0.0.4](https://github.com/wheregmis/dioxus-provider/compare/dioxus-provider-v0.0.3...dioxus-provider-v0.0.4) - 2025-06-30

### <!-- 3 -->Other

- Add Dependabot config and enhance release-plz settings
- Create FUNDING.yml
- Prefix composed provider result variables for uniqueness
- Update README with new features and examples
- Remove macro-based dependency injection support
- Refactor composable provider demo for CSS and structure
- Switch from tokio to futures join and platform-specific sleep
- Add composable provider support and demo example
- Update dependencies and refactor error handling in dependency injection
- Unify provider parameter handling with IntoProviderParam

## [0.0.3](https://github.com/wheregmis/dioxus-provider/compare/dioxus-provider-v0.0.2...dioxus-provider-v0.0.3) - 2025-06-24

### Other

- Re-invalidate optimistic cache keys on mutation rollback
- Update README.md
- Update README.md
- Mutation support
- initial mutation support

## [0.1.1](https://github.com/wheregmis/dioxus-provider/compare/dioxus-provider-macros-v0.1.0...dioxus-provider-macros-v0.1.1) - 2025-06-24

### Other

- Mutation support
- initial mutation support

## [0.0.2](https://github.com/wheregmis/dioxus-provider/compare/dioxus-provider-v0.0.1...dioxus-provider-v0.0.2) - 2025-06-22

### Other

- Update README with development warning and clean up example

## [0.0.1] - 2024-12-19

### Added
- **Global Provider System**: Manage application-wide data without nesting context providers
- **Declarative `#[provider]` Macro**: Define data sources with a simple attribute macro
- **Intelligent Caching Strategies**:
  - **Stale-While-Revalidate (SWR)**: Serve stale data instantly while fetching fresh data in the background
  - **Time-to-Live (TTL) Cache Expiration**: Automatically evict cached data after a configured duration
- **Automatic Refresh**: Keep data fresh with interval-based background refetching
- **Parameterized Queries**: Create providers that depend on dynamic arguments (e.g., fetching user data by ID)
- **Manual Cache Control**: Hooks to manually invalidate cached data or clear the entire cache
- **Cross-Platform Support**: Works seamlessly on both Desktop and Web (WASM)
- **Minimal Boilerplate**: Get started in minutes with intuitive hooks and macros

### Features
- `use_provider` hook for consuming provider data
- `use_invalidate_provider` hook for manual cache invalidation
- `use_clear_provider_cache` hook for clearing entire cache
