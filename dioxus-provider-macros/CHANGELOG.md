# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.3](https://github.com/wheregmis/dioxus-provider/compare/dioxus-provider-macros-v0.1.2...dioxus-provider-macros-v0.1.3) - 2025-10-29

### <!-- 3 -->Other

- Refactor types_equal to use structural equality
- unify optimistic_mutation with mutation
- automatic mutation
- Multiargument Support for mutation
- much cleaner api
- some clippy fixes

## [0.1.2](https://github.com/wheregmis/dioxus-provider/compare/dioxus-provider-macros-v0.1.1...dioxus-provider-macros-v0.1.2) - 2025-06-30

### <!-- 3 -->Other

- Prefix composed provider result variables for uniqueness
- Add validation for provider composition requirements
- Remove macro-based dependency injection support
- Clone parameters in async blocks for provider composition
- Switch from tokio to futures join and platform-specific sleep
- Add composable provider support and demo example
- Update dependencies and refactor error handling in dependency injection
