//! Common types and aliases used throughout dioxus-provider

/// Common trait bounds for provider parameters
pub trait ProviderParamBounds:
    Clone + PartialEq + std::hash::Hash + std::fmt::Debug + Send + Sync + 'static
{
}
impl<T> ProviderParamBounds for T where
    T: Clone + PartialEq + std::hash::Hash + std::fmt::Debug + Send + Sync + 'static
{
}

/// Common trait bounds for provider output types
pub trait ProviderOutputBounds: Clone + PartialEq + Send + Sync + 'static {}
impl<T> ProviderOutputBounds for T where T: Clone + PartialEq + Send + Sync + 'static {}

/// Common trait bounds for provider error types
pub trait ProviderErrorBounds: Clone + PartialEq + Send + Sync + 'static {}
impl<T> ProviderErrorBounds for T where T: Clone + PartialEq + Send + Sync + 'static {}
