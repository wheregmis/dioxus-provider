//! Parameter normalization utilities for dioxus-provider

use std::fmt::Debug;
use std::hash::Hash;

/// Trait for normalizing different parameter formats to work with providers
///
/// This trait allows the `use_provider` hook to accept parameters in different formats:
/// - `()` for no parameters
/// - `(param,)` for single parameter in tuple (e.g., `(42,)`)
/// - Common primitive types directly (e.g., `42`, `"foo".to_string()`)
/// - Custom types that implement the required bounds directly
///
/// # Custom Parameter Types
///
/// Custom types can be used directly as provider parameters by implementing the required bounds:
/// - `Clone + PartialEq + Hash + Debug + Send + Sync + 'static`
///
/// Use the `provider_param!` macro for convenience:
///
/// ```rust,ignore
/// use dioxus_provider::{prelude::*, provider_param};
///
/// #[derive(Clone, PartialEq, Eq, Hash, Debug)]
/// struct UserId(u32);
/// provider_param!(UserId);  // Enable direct usage
///
/// #[provider]
/// async fn fetch_user(user_id: UserId) -> Result<User, String> { todo!() }
///
/// // Now you can use it directly:
/// let user = use_provider(fetch_user(), UserId(42));
/// ```
///
/// # Usage and Ambiguity
///
/// - If your provider expects a single parameter, you can pass it directly or as a single-element tuple.
/// - **Note:** Tuple syntax `(param,)` has priority over direct parameter syntax for types that implement both.
/// - For string parameters, both `String` and `&str` are supported directly.
///
/// # Examples
///
/// ```rust,ignore
/// use dioxus_provider::prelude::*;
///
/// #[provider]
/// async fn fetch_user(user_id: u32) -> Result<User, String> { todo!() }
///
/// // All of these are valid:
/// let user = use_provider(fetch_user(), 42);      // direct primitive
/// let user = use_provider(fetch_user(), (42,));   // single-element tuple
/// let user = use_provider(fetch_user(), "foo".to_string()); // String
/// ```
pub trait IntoProviderParam {
    /// The target parameter type after conversion
    type Param: Clone + PartialEq + Hash + Debug + Send + Sync + 'static;

    /// Convert the input into the parameter format expected by the provider
    fn into_param(self) -> Self::Param;
}

/// Sealed trait to control which types can be used directly as provider parameters
///
/// This prevents conflicts between the blanket implementation and the tuple implementation.
/// Types that implement this trait (along with the required bounds) can be used directly
/// as provider parameters without requiring tuple wrappers.
pub mod sealed {
    pub trait DirectParam {}

    // Implement for common primitive types
    impl DirectParam for u8 {}
    impl DirectParam for u16 {}
    impl DirectParam for u32 {}
    impl DirectParam for u64 {}
    impl DirectParam for u128 {}
    impl DirectParam for usize {}
    impl DirectParam for i8 {}
    impl DirectParam for i16 {}
    impl DirectParam for i32 {}
    impl DirectParam for i64 {}
    impl DirectParam for i128 {}
    impl DirectParam for isize {}
    impl DirectParam for f32 {}
    impl DirectParam for f64 {}
    impl DirectParam for bool {}
    impl DirectParam for char {}
    impl DirectParam for String {}
    impl DirectParam for &str {}
}

// Implementation for no parameters: () -> ()
impl IntoProviderParam for () {
    type Param = ();

    fn into_param(self) -> Self::Param {
        // nothing needed
    }
}

// Implementation for tuple parameters: (Param,) -> Param
// This has higher priority than the blanket implementation due to Rust's orphan rules
impl<T> IntoProviderParam for (T,)
where
    T: Clone + PartialEq + Hash + Debug + Send + Sync + 'static,
{
    type Param = T;

    fn into_param(self) -> Self::Param {
        self.0
    }
}

// Blanket implementation for direct parameters
// This allows any type that implements the required bounds AND the sealed trait
// to be used directly as a provider parameter
impl<T> IntoProviderParam for T
where
    T: Clone + PartialEq + Hash + Debug + Send + Sync + 'static + sealed::DirectParam,
{
    type Param = T;

    fn into_param(self) -> Self::Param {
        self
    }
}

/// Macro to enable a custom type to be used directly as a provider parameter
///
/// This macro implements the sealed `DirectParam` trait for your type, allowing it
/// to be used with the blanket `IntoProviderParam` implementation.
///
/// # Requirements
///
/// Your type must implement:
/// - `Clone + PartialEq + Hash + Debug + Send + Sync + 'static`
///
/// # Example
///
/// ```rust,ignore
/// use dioxus_provider::{prelude::*, provider_param};
///
/// #[derive(Clone, PartialEq, Eq, Hash, Debug)]
/// struct UserId(u32);
///
/// #[derive(Clone, PartialEq, Eq, Hash, Debug)]
/// struct ProductId(String);
///
/// // Enable direct usage as provider parameters
/// provider_param!(UserId);
/// provider_param!(ProductId);
///
/// #[provider]
/// async fn fetch_user(user_id: UserId) -> Result<User, String> { todo!() }
///
/// #[provider]
/// async fn fetch_product(product_id: ProductId) -> Result<Product, String> { todo!() }
///
/// // Now you can use them directly:
/// let user = use_provider(fetch_user(), UserId(42));
/// let product = use_provider(fetch_product(), ProductId("abc".to_string()));
/// ```
#[macro_export]
macro_rules! provider_param {
    ($type:ty) => {
        impl $crate::param_utils::sealed::DirectParam for $type {}
    };
}
