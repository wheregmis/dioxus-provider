//! State: Async state enum for dioxus-provider
//!
//! This module provides the `State` enum and the `AsyncState` trait for working
//! with asynchronous operations in dioxus-provider.

use dioxus::core::Task;

/// Common trait for async state types that represent loading, success, and error states
///
/// This trait provides a unified interface for working with different async state types
/// in dioxus-provider, such as `State` and `MutationState`.
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
pub enum State<T, E> {
    /// The operation is currently loading
    Loading { task: Task },
    /// The operation completed successfully with data
    Success(T),
    /// The operation failed with an error
    Error(E),
}

impl<T, E> AsyncState for State<T, E> {
    type Data = T;
    type Error = E;

    fn is_loading(&self) -> bool {
        matches!(self, State::Loading { task: _ })
    }

    fn is_success(&self) -> bool {
        matches!(self, State::Success(_))
    }

    fn is_error(&self) -> bool {
        matches!(self, State::Error(_))
    }

    fn data(&self) -> Option<&T> {
        match self {
            State::Success(data) => Some(data),
            _ => None,
        }
    }

    fn error(&self) -> Option<&E> {
        match self {
            State::Error(error) => Some(error),
            _ => None,
        }
    }
}

impl<T, E> State<T, E> {
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

    /// Maps a State<T, E> to State<U, E> by applying a function to the contained data if successful.
    pub fn map<U, F>(self, op: F) -> State<U, E>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            State::Success(data) => State::Success(op(data)),
            State::Error(e) => State::Error(e),
            State::Loading { task } => State::Loading { task },
        }
    }

    /// Maps a State<T, E> to State<T, F> by applying a function to the contained error if failed.
    pub fn map_err<F, O>(self, op: O) -> State<T, F>
    where
        O: FnOnce(E) -> F,
    {
        match self {
            State::Success(data) => State::Success(data),
            State::Error(e) => State::Error(op(e)),
            State::Loading { task } => State::Loading { task },
        }
    }

    /// Chains a State<T, E> to State<U, E> by applying a function to the contained data if successful.
    pub fn and_then<U, F>(self, op: F) -> State<U, E>
    where
        F: FnOnce(T) -> State<U, E>,
    {
        match self {
            State::Success(data) => op(data),
            State::Error(e) => State::Error(e),
            State::Loading { task } => State::Loading { task },
        }
    }
}
