//! Common types and trait bounds used throughout dioxus-provider
//!
//! This module defines the foundational trait bounds that enable dioxus-provider to work
//! seamlessly across different platforms (web, desktop, mobile) while maintaining type safety.

/// Common trait bounds for provider parameters
///
/// Provider parameters must satisfy these bounds to enable:
/// - **Clone**: Parameters are cloned when passed to async functions and stored in cache keys
/// - **PartialEq**: Required for comparing parameters to determine cache key equality
/// - **Hash**: Enables efficient cache key generation from parameter values
/// - **Debug**: Provides better error messages and logging output
/// - **Send + Sync**: Allows parameters to be safely shared across async contexts
/// - **'static**: Ensures parameters can be stored in global caches and async tasks
///
/// ## Example
///
/// ```rust
/// // Simple types automatically satisfy these bounds:
/// use dioxus_provider::prelude::*;
///
/// #[provider]
/// async fn fetch_user(user_id: u32) -> Result<String, String> {
///     // u32 implements ProviderParamBounds automatically
///     Ok(format!("User {}", user_id))
/// }
///
/// // Custom types need to derive the necessary traits:
/// #[derive(Clone, PartialEq, Eq, Hash, Debug)]
/// struct UserId(u32);
///
/// #[provider]
/// async fn fetch_user_custom(id: UserId) -> Result<String, String> {
///     // UserId now implements ProviderParamBounds
///     Ok(format!("User {}", id.0))
/// }
/// ```
pub trait ProviderParamBounds:
    Clone + PartialEq + std::hash::Hash + std::fmt::Debug + Send + Sync + 'static
{
}
impl<T> ProviderParamBounds for T where
    T: Clone + PartialEq + std::hash::Hash + std::fmt::Debug + Send + Sync + 'static
{
}

/// Common trait bounds for provider output types
///
/// Provider outputs must satisfy these bounds to enable:
/// - **Clone**: Results are cloned when cached and served to multiple components
/// - **PartialEq**: Allows change detection to avoid unnecessary re-renders
/// - **Send + Sync**: Enables safe sharing of results across async contexts
/// - **'static**: Required for storing results in global caches and signals
///
/// ## Example
///
/// ```rust
/// use dioxus_provider::prelude::*;
///
/// // Simple types work out of the box:
/// #[provider]
/// async fn fetch_count() -> Result<i32, String> {
///     Ok(42)
/// }
///
/// // Custom types need to derive Clone and PartialEq:
/// #[derive(Clone, PartialEq)]
/// pub struct User {
///     id: u32,
///     name: String,
/// }
///
/// #[provider]
/// async fn fetch_user(id: u32) -> Result<User, String> {
///     Ok(User { id, name: "Alice".to_string() })
/// }
/// ```
pub trait ProviderOutputBounds: Clone + PartialEq + Send + Sync + 'static {}
impl<T> ProviderOutputBounds for T where T: Clone + PartialEq + Send + Sync + 'static {}

/// Common trait bounds for provider error types
///
/// Provider errors must satisfy these bounds to enable:
/// - **Clone**: Errors are cloned when cached and displayed to users
/// - **PartialEq**: Enables error comparison and change detection
/// - **Send + Sync**: Allows safe propagation of errors across async boundaries
/// - **'static**: Required for storing errors in caches and error states
///
/// ## Example
///
/// ```rust
/// use dioxus_provider::prelude::*;
/// use thiserror::Error;
///
/// // String works as a simple error type:
/// #[provider]
/// async fn simple_provider() -> Result<String, String> {
///     Err("Something went wrong".to_string())
/// }
///
/// // Custom error types provide better structure:
/// #[derive(Error, Debug, Clone, PartialEq)]
/// pub enum MyError {
///     #[error("Network error: {0}")]
///     Network(String),
///     #[error("Not found")]
///     NotFound,
/// }
///
/// #[provider]
/// async fn typed_provider() -> Result<String, MyError> {
///     Err(MyError::NotFound)
/// }
/// ```
pub trait ProviderErrorBounds: Clone + PartialEq + Send + Sync + 'static {}
impl<T> ProviderErrorBounds for T where T: Clone + PartialEq + Send + Sync + 'static {}
