//! Shopping Cart Domain Models
//!
//! This module contains all data structures related to the shopping cart
//! business domain.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

// =============================================================================
// Cart Domain Models
// =============================================================================

/// Returns the default quantity (1) for cart items
fn default_quantity() -> u32 {
    1
}

/// Represents an item in the shopping cart
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CartItem {
    /// Name of the product
    pub name: String,

    /// Quantity of this item (defaults to 1)
    #[serde(default = "default_quantity")]
    pub quantity: u32,

    /// Captures any extra fields (e.g., price, description) dynamically
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// Input for the add_to_cart tool
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddToCartInput {
    /// List of items to add to the cart
    pub items: Vec<CartItem>,

    /// Optional cart identifier
    pub cart_id: Option<String>,
}

/// Input for the checkout tool
#[derive(Debug, Deserialize)]
pub struct CheckoutInput {
    /// Optional cart identifier
    #[serde(rename = "cartId")]
    pub cart_id: Option<String>,
}

/// Response for cart synchronization operations
#[derive(Serialize)]
pub struct SyncResponse {
    /// Status of the operation
    pub status: String,

    /// Cart identifier
    #[serde(rename = "cartId")]
    pub cart_id: String,
}
