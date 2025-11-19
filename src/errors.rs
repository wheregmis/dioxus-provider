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

/// Errors specific to user operations
#[derive(Error, Debug, Clone, PartialEq)]
pub enum UserError {
    /// User not found
    #[error("User not found: {id}")]
    NotFound { id: u32 },

    /// User account is suspended
    #[error("User suspended: {reason}")]
    Suspended { reason: String },

    /// User account is deleted
    #[error("User deleted: {id}")]
    Deleted { id: u32 },

    /// Permission denied for user operation
    #[error("Permission denied for user {user_id}: {action}")]
    PermissionDenied { user_id: u32, action: String },

    /// User validation errors
    #[error("User validation failed: {field}: {reason}")]
    ValidationFailed { field: String, reason: String },

    /// Wraps provider errors
    #[error("Provider error: {0}")]
    Provider(#[from] ProviderError),
}

/// Errors specific to API operations
#[derive(Error, Debug, Clone, PartialEq)]
pub enum ApiError {
    /// HTTP status errors
    #[error("HTTP {status}: {message}")]
    HttpStatus { status: u16, message: String },

    /// JSON parsing errors
    #[error("JSON parsing failed: {0}")]
    JsonParsing(String),

    /// Request building errors
    #[error("Request building failed: {0}")]
    RequestBuilding(String),

    /// Response processing errors
    #[error("Response processing failed: {0}")]
    ResponseProcessing(String),

    /// API endpoint not found
    #[error("API endpoint not found: {endpoint}")]
    EndpointNotFound { endpoint: String },

    /// API version mismatch
    #[error("API version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: String, actual: String },

    /// Wraps provider errors
    #[error("Provider error: {0}")]
    Provider(#[from] ProviderError),
}

/// Errors specific to database operations
#[derive(Error, Debug, Clone, PartialEq)]
pub enum DatabaseError {
    /// Connection errors
    #[error("Database connection failed: {0}")]
    Connection(String),

    /// Query execution errors
    #[error("Query execution failed: {query}: {error}")]
    QueryExecution { query: String, error: String },

    /// Transaction errors
    #[error("Transaction failed: {0}")]
    Transaction(String),

    /// Migration errors
    #[error("Database migration failed: {0}")]
    Migration(String),

    /// Constraint violation errors
    #[error("Database constraint violation: {constraint}: {details}")]
    ConstraintViolation { constraint: String, details: String },

    /// Record not found
    #[error("Record not found: {table}: {id}")]
    RecordNotFound { table: String, id: String },

    /// Wraps provider errors
    #[error("Provider error: {0}")]
    Provider(#[from] ProviderError),
}

/// Convenience type alias for Results with ProviderError
pub type ProviderResult<T> = Result<T, ProviderError>;

/// Convenience type alias for Results with UserError
pub type UserResult<T> = Result<T, UserError>;

/// Convenience type alias for Results with ApiError
pub type ApiResult<T> = Result<T, ApiError>;

/// Convenience type alias for Results with DatabaseError
pub type DatabaseResult<T> = Result<T, DatabaseError>;

impl From<String> for ProviderError {
    fn from(error: String) -> Self {
        ProviderError::Generic(error)
    }
}

impl From<&str> for ProviderError {
    fn from(error: &str) -> Self {
        ProviderError::Generic(error.to_string())
    }
}

impl From<ProviderError> for String {
    fn from(error: ProviderError) -> Self {
        error.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_error_display() {
        let error = ProviderError::InvalidInput("test input".to_string());
        assert_eq!(error.to_string(), "Invalid input: test input");
    }

    #[test]
    fn test_user_error_with_provider_error() {
        let provider_error = ProviderError::Network("connection failed".to_string());
        let user_error = UserError::Provider(provider_error);
        assert_eq!(
            user_error.to_string(),
            "Provider error: Network error: connection failed"
        );
    }

    #[test]
    fn test_api_error_http_status() {
        let error = ApiError::HttpStatus {
            status: 404,
            message: "Not Found".to_string(),
        };
        assert_eq!(error.to_string(), "HTTP 404: Not Found");
    }

    #[test]
    fn test_database_error_constraint_violation() {
        let error = DatabaseError::ConstraintViolation {
            constraint: "unique_email".to_string(),
            details: "Email already exists".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "Database constraint violation: unique_email: Email already exists"
        );
    }
}
