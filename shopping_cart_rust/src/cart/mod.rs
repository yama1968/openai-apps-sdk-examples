//! Shopping Cart Domain Module
//!
//! This module contains all shopping cart business logic, including:
//! - Domain models (CartItem, inputs, responses)
//! - Business logic helpers (cart operations, formatting)
//! - Application state management
//! - REST API handlers

pub mod handlers;
pub mod helpers;
pub mod models;
pub mod state;

// Re-export commonly used types for convenience
pub use handlers::routes;
pub use state::{AppState, SharedState};
