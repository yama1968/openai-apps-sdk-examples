//! MCP Protocol Helpers
//!
//! This module contains helper functions for JSON-RPC communication
//! and OpenAI widget metadata construction.

use serde_json::{json, Value};

/// Constructs the metadata required by the OpenAI widget system.
///
/// # Arguments
///
/// * `session_id` - Optional identifier to link tool calls to a specific widget session.
pub fn widget_meta(session_id: Option<&str>) -> Value {
    let mut meta = json!({
        "openai/outputTemplate": super::models::WIDGET_TEMPLATE_URI,
        "openai/toolInvocation/invoking": "Preparing shopping cart",
        "openai/toolInvocation/invoked": "Shopping cart ready",
        "openai/widgetAccessible": true,
    });

    if let Some(id) = session_id {
        meta["openai/widgetSessionId"] = json!(id);
    }

    meta
}

/// Builds a JSON-RPC 2.0 success response.
///
/// # Arguments
///
/// * `id` – The request identifier that must be echoed back.
/// * `result` – The payload representing the successful outcome.
///
/// # Returns
///
/// A `serde_json::Value` shaped as a JSON-RPC success envelope.
pub fn rpc_success(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
}

/// Builds a JSON-RPC 2.0 error response.
///
/// # Arguments
///
/// * `id` – The request identifier (or `null` if unavailable).
/// * `code` – The JSON-RPC error code (e.g., -32601 for method not found).
/// * `message` – Human-readable description of the error.
///
/// # Returns
///
/// A `serde_json::Value` shaped as a JSON-RPC error envelope.
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
