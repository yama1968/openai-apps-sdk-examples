//! Helper utilities for the shopping‑cart Rust backend
//!
//! This module houses small, pure functions that are used across the
//! application (JSON‑RPC envelope construction, widget metadata, cart
//! management, etc.). Keeping them separated from the data models makes the
//! codebase easier to navigate and test.

use serde_json::{json, Value};

use uuid::Uuid;

/// Constructs the metadata required by the OpenAI widget system.
///
/// The fields are defined by the SDK specification:
/// - `openai/outputTemplate` – URI of the widget HTML.
/// - `openai/toolInvocation/invoking` / `invoked` – human readable
///   messages for the tool lifecycle.
/// - `openai/widgetAccessible` – indicates the widget may be rendered.
pub fn widget_meta() -> Value {
    json!({
        "openai/outputTemplate": super::model::WIDGET_TEMPLATE_URI,
        "openai/toolInvocation/invoking": "Preparing shopping cart",
        "openai/toolInvocation/invoked": "Shopping cart ready",
        "openai/widgetAccessible": true,
    })
}

/// Builds a JSON‑RPC 2.0 success response.
///
/// # Arguments
///
/// * `id` – The request identifier that must be echoed back.
/// * `result` – The payload representing the successful outcome.
///
/// # Returns
///
/// A `serde_json::Value` shaped as a JSON‑RPC success envelope.
pub fn rpc_success(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
}

/// Builds a JSON‑RPC 2.0 error response.
///
/// # Arguments
///
/// * `id` – The request identifier (or `null` if unavailable).
/// * `code` – The JSON‑RPC error code (e.g., -32601 for method not found).
/// * `message` – Human‑readable description of the error.
///
/// # Returns
///
/// A `serde_json::Value` shaped as a JSON‑RPC error envelope.
pub fn rpc_error(id: Value, code: i32, message: impl Into<String>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message.into(),
        }
    })
}

/// Returns the provided `cart_id` or creates a new UUID string when `None`.
///
/// This guarantees that every cart operation works with a non‑empty identifier.
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
/// This function mutates `cart_items` in‑place.
pub fn update_cart_with_new_items(
    cart_items: &mut Vec<super::model::CartItem>,
    new_items: Vec<super::model::CartItem>,
) {
    for incoming in new_items {
        if let Some(existing) = cart_items.iter_mut().find(|i| i.name == incoming.name) {
            // Aggregate quantities.
            existing.quantity += incoming.quantity;
        } else {
            // Insert a brand‑new item.
            cart_items.push(incoming);
        }
    }
}

/// Produces a human‑readable one‑line summary for a list of cart items.
///
/// Example output: `"2x Apple, 1x Banana"`.
pub fn format_item_summary(items: &[super::model::CartItem]) -> String {
    items
        .iter()
        .map(|i| format!("{}x {}", i.quantity, i.name))
        .collect::<Vec<_>>()
        .join(", ")
}
