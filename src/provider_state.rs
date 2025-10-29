//! ProviderState: Async state enum for dioxus-provider
//!
//! This module provides the `ProviderState` enum and the `AsyncState` trait for working
//! with asynchronous operations in dioxus-provider.

use dioxus::core::Task;

/// Common trait for async state types that represent loading, success, and error states
///
/// This trait provides a unified interface for working with different async state types
/// in dioxus-provider, such as `ProviderState` and `MutationState`.
pub trait AsyncState {
    /// The type of successful data
    type Data;
    /// The type of error
    type Error;

    /// Returns true if the state is currently loading
    fn is_loading(&self) -> bool;

    /// Returns true if the state contains successful data
    fn is_success(&self) -> bool;

    /// Returns true if the state contains an error
    fn is_error(&self) -> bool;

    /// Returns the data if successful, None otherwise
    fn data(&self) -> Option<&Self::Data>;

    /// Returns the error if failed, None otherwise
    fn error(&self) -> Option<&Self::Error>;
}

/// Represents the state of an async operation
#[derive(Clone, PartialEq, Debug)]
pub enum ProviderState<T, E> {
    /// The operation is currently loading
    Loading { task: Task },
    /// The operation completed successfully with data
    Success(T),
    /// The operation failed with an error
    Error(E),
}

impl<T, E> AsyncState for ProviderState<T, E> {
    type Data = T;
    type Error = E;

    fn is_loading(&self) -> bool {
        matches!(self, ProviderState::Loading { task: _ })
    }

    fn is_success(&self) -> bool {
        matches!(self, ProviderState::Success(_))
    }

    fn is_error(&self) -> bool {
        matches!(self, ProviderState::Error(_))
    }

    fn data(&self) -> Option<&T> {
        match self {
            ProviderState::Success(data) => Some(data),
            _ => None,
        }
    }

    fn error(&self) -> Option<&E> {
        match self {
            ProviderState::Error(error) => Some(error),
            _ => None,
        }
    }
}

impl<T, E> ProviderState<T, E> {
    /// Returns true if the state is currently loading
    pub fn is_loading(&self) -> bool {
        <Self as AsyncState>::is_loading(self)
    }

    /// Returns true if the state contains successful data
    pub fn is_success(&self) -> bool {
        <Self as AsyncState>::is_success(self)
    }

    /// Returns true if the state contains an error
    pub fn is_error(&self) -> bool {
        <Self as AsyncState>::is_error(self)
    }

    /// Returns the data if successful, None otherwise
    pub fn data(&self) -> Option<&T> {
        <Self as AsyncState>::data(self)
    }

    /// Returns the error if failed, None otherwise
    pub fn error(&self) -> Option<&E> {
        <Self as AsyncState>::error(self)
    }

    /// Maps a ProviderState<T, E> to ProviderState<U, E> by applying a function to the contained data if successful.
    pub fn map<U, F>(self, op: F) -> ProviderState<U, E>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            ProviderState::Success(data) => ProviderState::Success(op(data)),
            ProviderState::Error(e) => ProviderState::Error(e),
            ProviderState::Loading { task } => ProviderState::Loading { task },
        }
    }

    /// Maps a ProviderState<T, E> to ProviderState<T, F> by applying a function to the contained error if failed.
    pub fn map_err<F, O>(self, op: O) -> ProviderState<T, F>
    where
        O: FnOnce(E) -> F,
    {
        match self {
            ProviderState::Success(data) => ProviderState::Success(data),
            ProviderState::Error(e) => ProviderState::Error(op(e)),
            ProviderState::Loading { task } => ProviderState::Loading { task },
        }
    }

    /// Chains a ProviderState<T, E> to ProviderState<U, E> by applying a function to the contained data if successful.
    pub fn and_then<U, F>(self, op: F) -> ProviderState<U, E>
    where
        F: FnOnce(T) -> ProviderState<U, E>,
    {
        match self {
            ProviderState::Success(data) => op(data),
            ProviderState::Error(e) => ProviderState::Error(e),
            ProviderState::Loading { task } => ProviderState::Loading { task },
        }
    }
}
