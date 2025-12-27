//! Shopping Cart Business Logic Helpers
//!
//! This module contains helper functions for cart operations and formatting.

use super::models::CartItem;

use uuid::Uuid;

/// Returns the provided `cart_id` or falls back to the `default_id`.
///
/// This allows "unnamed" cart operations to stick to a single default session.
pub fn get_or_default_cart_id(cart_id: Option<String>, default_id: &str) -> String {
    cart_id.unwrap_or_else(|| default_id.to_string())
}

/// Resolves the session ID from the `Cookie` header or generates a new one.
///
/// Use this to implement sticky default carts.
///
/// # Returns
/// (session_id, is_new)
pub fn resolve_session_id(headers: &axum::http::HeaderMap) -> (String, bool) {
    if let Some(cookie_header) = headers.get(axum::http::header::COOKIE) {
        if let Ok(cookie_str) = cookie_header.to_str() {
            // Simple manual parsing to avoid extra dependencies for now
            // Format: name=value; name2=value2
            for part in cookie_str.split(';') {
                let part = part.trim();
                if part.starts_with("cart_session=") {
                    let val = part.trim_start_matches("cart_session=");
                    if !val.is_empty() {
                        return (val.to_string(), false);
                    }
                }
            }
        }
    }

    (Uuid::new_v4().simple().to_string(), true)
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
