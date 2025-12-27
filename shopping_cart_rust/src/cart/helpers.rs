//! Shopping Cart Business Logic Helpers
//!
//! This module contains helper functions for cart operations and formatting.

use super::models::CartItem;
use uuid::Uuid;

/// Returns the provided `cart_id` or creates a new UUID string when `None`.
///
/// This guarantees that every cart operation works with a non-empty identifier.
pub fn get_or_create_cart_id(cart_id: Option<String>) -> String {
    cart_id.unwrap_or_else(|| Uuid::new_v4().simple().to_string())
}

/// Merges `new_items` into `cart_items`, aggregating quantities for existing
/// entries and inserting brand new ones.
///
/// # Behaviour
///
/// * If an item with the same name already exists, its `quantity` is
///   increased by the incoming quantity.
/// * Extra fields (`extra` hashmap) are **not** merged – the function mirrors the
///   Python reference implementation, which only updates quantity.
///
/// This function mutates `cart_items` in-place.
pub fn update_cart_with_new_items(cart_items: &mut Vec<CartItem>, new_items: Vec<CartItem>) {
    for incoming in new_items {
        if let Some(existing) = cart_items.iter_mut().find(|i| i.name == incoming.name) {
            // Aggregate quantities.
            existing.quantity += incoming.quantity;
        } else {
            // Insert a brand-new item.
            cart_items.push(incoming);
        }
    }
}

/// Produces a human-readable one-line summary for a list of cart items.
///
/// Example output: `"2x Apple, 1x Banana"`.
pub fn format_item_summary(items: &[CartItem]) -> String {
    items
        .iter()
        .map(|i| format!("{}x {}", i.quantity, i.name))
        .collect::<Vec<_>>()
        .join(", ")
}
