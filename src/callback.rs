//! Callback traits for providers and mutations
//!
//! This module provides the `ProviderCallback` trait which allows any async function
//! to be used with `use_provider`. Similar to Dioxus's `ActionCallback` trait.

use std::collections::hash_map::DefaultHasher;
use std::future::Future;
use std::hash::{Hash, Hasher};

/// A trait for async functions that can be used as providers.
///
/// This trait is automatically implemented for async functions that return `Result<O, E>`.
/// It supports functions with 0-4 parameters.
///
/// # Example
///
/// ```rust,ignore
/// async fn fetch_user(id: u32) -> Result<User, Error> {
///     // fetch user...
/// }
///
/// // fetch_user automatically implements ProviderCallback<(u32,), Error>
/// let provider = use_provider(fetch_user);
/// provider.call(123);
/// ```
pub trait ProviderCallback<M, E>: Clone + 'static {
    /// The input type for the callback (tuple of parameters)
    type Input: Clone + Hash + 'static;
    /// The output type on success
    type Output: Clone + 'static;

    /// Call the async function with the given input
    fn call(&self, input: Self::Input) -> impl Future<Output = Result<Self::Output, E>> + 'static;

    /// Generate a cache key for the given input
    ///
    /// The default implementation hashes the function's type ID and the input parameters.
    fn cache_key(&self, input: &Self::Input) -> String {
        format!("{}:{}", self.root_key(), self.args_key(input))
    }

    /// Generate a type-safe cache key for the given input
    fn ty_key(&self, input: &Self::Input) -> crate::cache::TypedKey<Self::Output> {
        crate::cache::TypedKey::new(self.cache_key(input))
    }

    /// Get the root key prefix for this provider function.
    ///
    /// This is used for invalidating all entries for a specific provider.
    fn root_key(&self) -> String {
        let mut hasher = DefaultHasher::new();
        std::any::TypeId::of::<Self>().hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Generate the arguments part of the key
    fn args_key(&self, input: &Self::Input) -> String {
        let mut hasher = DefaultHasher::new();
        input.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

// Implementation for 0 parameters: FnMut() -> Future<Output = Result<O, E>>
impl<F, O, G, E> ProviderCallback<(O,), E> for F
where
    F: Fn() -> G + Clone + 'static,
    G: Future<Output = Result<O, E>> + 'static,
    O: Clone + 'static,
    E: 'static,
{
    type Input = ();
    type Output = O;

    fn call(&self, _input: Self::Input) -> impl Future<Output = Result<Self::Output, E>> + 'static {
        (self)()
    }
}

// Implementation for 1 parameter: FnMut(A) -> Future<Output = Result<O, E>>
impl<F, O, A, G, E> ProviderCallback<(A, O), E> for F
where
    F: Fn(A) -> G + Clone + 'static,
    G: Future<Output = Result<O, E>> + 'static,
    A: Clone + Hash + 'static,
    O: Clone + 'static,
    E: 'static,
{
    type Input = (A,);
    type Output = O;

    fn call(&self, input: Self::Input) -> impl Future<Output = Result<Self::Output, E>> + 'static {
        let (a,) = input;
        (self)(a)
    }
}

// Implementation for 2 parameters: FnMut(A, B) -> Future<Output = Result<O, E>>
impl<F, O, A, B, G, E> ProviderCallback<(A, B, O), E> for F
where
    F: Fn(A, B) -> G + Clone + 'static,
    G: Future<Output = Result<O, E>> + 'static,
    A: Clone + Hash + 'static,
    B: Clone + Hash + 'static,
    O: Clone + 'static,
    E: 'static,
{
    type Input = (A, B);
    type Output = O;

    fn call(&self, input: Self::Input) -> impl Future<Output = Result<Self::Output, E>> + 'static {
        let (a, b) = input;
        (self)(a, b)
    }
}

// Implementation for 3 parameters: FnMut(A, B, C) -> Future<Output = Result<O, E>>
impl<F, O, A, B, C, G, E> ProviderCallback<(A, B, C, O), E> for F
where
    F: Fn(A, B, C) -> G + Clone + 'static,
    G: Future<Output = Result<O, E>> + 'static,
    A: Clone + Hash + 'static,
    B: Clone + Hash + 'static,
    C: Clone + Hash + 'static,
    O: Clone + 'static,
    E: 'static,
{
    type Input = (A, B, C);
    type Output = O;

    fn call(&self, input: Self::Input) -> impl Future<Output = Result<Self::Output, E>> + 'static {
        let (a, b, c) = input;
        (self)(a, b, c)
    }
}

// Implementation for 4 parameters: FnMut(A, B, C, D) -> Future<Output = Result<O, E>>
impl<F, O, A, B, C, D, G, E> ProviderCallback<(A, B, C, D, O), E> for F
where
    F: Fn(A, B, C, D) -> G + Clone + 'static,
    G: Future<Output = Result<O, E>> + 'static,
    A: Clone + Hash + 'static,
    B: Clone + Hash + 'static,
    C: Clone + Hash + 'static,
    D: Clone + Hash + 'static,
    O: Clone + 'static,
    E: 'static,
{
    type Input = (A, B, C, D);
    type Output = O;

    fn call(&self, input: Self::Input) -> impl Future<Output = Result<Self::Output, E>> + 'static {
        let (a, b, c, d) = input;
        (self)(a, b, c, d)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn no_params() -> Result<String, String> {
        Ok("hello".to_string())
    }

    async fn one_param(id: u32) -> Result<String, String> {
        Ok(format!("user_{}", id))
    }

    #[test]
    fn test_cache_key_uniqueness() {
        // Different inputs should produce different cache keys
        let key1 = one_param.cache_key(&(1,));
        let key2 = one_param.cache_key(&(2,));
        assert_ne!(key1, key2);

        // Same inputs should produce same cache keys
        let key3 = one_param.cache_key(&(1,));
        assert_eq!(key1, key3);
    }

    #[test]
    fn test_different_functions_different_keys() {
        // Different functions with same input should produce different keys
        let key1 = no_params.cache_key(&());
        
        async fn another_no_params() -> Result<String, String> {
            Ok("world".to_string())
        }
        let key2 = another_no_params.cache_key(&());
        
        assert_ne!(key1, key2);
    }
}

