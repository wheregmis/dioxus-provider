//! # Structured Error Types
//!
//! This module provides structured error types for common scenarios in data fetching
//! and provider operations. Using structured errors instead of generic `String` errors
//! provides better error handling, debugging, and type safety.
//!
//! ## Examples
//!
//! ### Using ProviderError for general provider failures:
//! ```rust,ignore
//! use dioxus_provider::{errors::ProviderError, prelude::*};
//!
//! #[derive(Clone, PartialEq)]
//! struct User;
//!
//! #[provider]
//! async fn fetch_user(user_id: u32) -> Result<User, ProviderError> {
//!     if user_id == 0 {
//!         return Err(ProviderError::InvalidInput("User ID cannot be zero".to_string()));
//!     }
//!     Ok(User)
//! }
//! ```
//!
//! ### Using custom domain-specific errors:
//! ```rust,ignore
//! use dioxus_provider::prelude::*;
//! use thiserror::Error;
//!
//! #[derive(Clone, PartialEq)]
//! struct UserProfile;
//!
//! #[derive(Error, Debug, Clone, PartialEq)]
//! pub enum UserError {
//!     #[error("User not found: {id}")]
//!     NotFound { id: u32 },
//!     #[error("User is suspended: {reason}")]
//!     Suspended { reason: String },
//! }
//!
//! #[provider]
//! async fn fetch_user_profile(user_id: u32) -> Result<UserProfile, UserError> {
//!     Ok(UserProfile)
//! }
//! ```

use thiserror::Error;

/// Common error types for provider operations
#[derive(Error, Debug, Clone, PartialEq)]
pub enum ProviderError {
    /// Invalid input parameters
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Network or HTTP errors
    #[error("Network error: {0}")]
    Network(String),

    /// External service errors  
    #[error("External service '{service}' error: {error}")]
    ExternalService { service: String, error: String },

    /// Data parsing or serialization errors
    #[error("Data parsing error: {0}")]
    DataParsing(String),

    /// Authentication errors
    #[error("Authentication failed: {0}")]
    Authentication(String),

    /// Authorization errors
    #[error("Authorization failed: {0}")]
    Authorization(String),

    /// Rate limiting errors
    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),

    /// Timeout errors
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Dependency injection errors
    #[error("Dependency injection failed: {0}")]
    DependencyInjection(String),

    /// Cache errors
    #[error("Cache error: {0}")]
    Cache(String),

    /// Generic provider errors for cases not covered above
    #[error("Provider error: {0}")]
    Generic(String),
}
