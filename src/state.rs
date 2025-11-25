//! State: Async state enum for dioxus-provider
//!
//! This module provides the `State` enum for working with asynchronous operations.

use dioxus::core::Task;

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

impl<T, E> State<T, E> {
    /// Returns true if the state is currently loading
    pub fn is_loading(&self) -> bool {
        matches!(self, State::Loading { task: _ })
    }

    /// Returns true if the state contains successful data
    pub fn is_success(&self) -> bool {
        matches!(self, State::Success(_))
    }

    /// Returns true if the state contains an error
    pub fn is_error(&self) -> bool {
        matches!(self, State::Error(_))
    }

    /// Returns the data if successful, None otherwise
    pub fn data(&self) -> Option<&T> {
        match self {
            State::Success(data) => Some(data),
            _ => None,
        }
    }

    /// Returns the error if failed, None otherwise
    pub fn error(&self) -> Option<&E> {
        match self {
            State::Error(error) => Some(error),
            _ => None,
        }
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

    /// Returns the contained success value or a provided default.
    ///
    /// # Example
    /// ```ignore
    /// let count = state.unwrap_or(0);
    /// ```
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            State::Success(data) => data,
            _ => default,
        }
    }

    /// Returns the contained success value or computes it from a closure.
    ///
    /// # Example
    /// ```ignore
    /// let count = state.unwrap_or_else(|| expensive_default());
    /// ```
    pub fn unwrap_or_else<F>(self, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        match self {
            State::Success(data) => data,
            _ => f(),
        }
    }

    /// Converts from State<T, E> to Option<T>, discarding loading and error states.
    ///
    /// # Example
    /// ```ignore
    /// if let Some(data) = state.ok() {
    ///     // use data
    /// }
    /// ```
    pub fn ok(self) -> Option<T> {
        match self {
            State::Success(data) => Some(data),
            _ => None,
        }
    }

    /// Converts from State<T, E> to Result<T, E>, treating Loading as an error.
    ///
    /// Returns Err with the provided loading_error if state is Loading.
    pub fn into_result(self, loading_error: E) -> Result<T, E> {
        match self {
            State::Success(data) => Ok(data),
            State::Error(e) => Err(e),
            State::Loading { .. } => Err(loading_error),
        }
    }

    /// Returns true if the state has resolved (either Success or Error, not Loading).
    pub fn is_resolved(&self) -> bool {
        !self.is_loading()
    }
}

impl<T: Clone, E> State<T, E> {
    /// Returns a clone of the contained success value or a provided default.
    ///
    /// Unlike `unwrap_or`, this method takes `&self` and clones the data.
    pub fn cloned_or(&self, default: T) -> T {
        match self {
            State::Success(data) => data.clone(),
            _ => default,
        }
    }

    /// Returns a clone of the contained success value or computes it from a closure.
    pub fn cloned_or_else<F>(&self, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        match self {
            State::Success(data) => data.clone(),
            _ => f(),
        }
    }
}

impl<T: Default, E> State<T, E> {
    /// Returns the contained success value or a default.
    ///
    /// # Example
    /// ```ignore
    /// let items: Vec<Item> = state.unwrap_or_default();
    /// ```
    pub fn unwrap_or_default(self) -> T {
        match self {
            State::Success(data) => data,
            _ => T::default(),
        }
    }
}
