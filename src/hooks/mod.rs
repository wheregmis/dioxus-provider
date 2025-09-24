//! Provider hooks and utilities for Dioxus applications

// Internal helper modules
mod swr;
mod tasks;
mod cache_mgmt;

// Main hooks implementation
mod main;

// Re-export everything from main
pub use main::*;