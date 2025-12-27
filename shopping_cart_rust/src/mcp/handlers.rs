//! MCP (Model Context Protocol) route handlers
//!
//! This module implements the Model Context Protocol handlers for the shopping cart application.
//! It exports `handle_tool_call` publicly to make it accessible for tests.

use super::{helpers::*, models::*};
use crate::cart::{helpers::*, models::*, state::*};
use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::post, Json, Router};
use serde_json::{json, Value};

/// Creates routes for MCP-related operations
pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/", post(handle_mcp).get(handle_mcp_sse))
        .route("/mcp", post(handle_mcp).get(handle_mcp_sse)) // Standard endpoint
        .route("/mcp/", post(handle_mcp).get(handle_mcp_sse)) // Trailing slash safety
}

/// Handle SSE (Server-Sent Events) handshake for GET requests
async fn handle_mcp_sse() -> impl IntoResponse {
    (
        [("content-type", "text/event-stream")],
        "event: endpoint\ndata: /mcp\n\n",
    )
}

/// Endpoint: POST /mcp
/// Handles the Model Context Protocol communication for POST requests.
async fn handle_mcp(
    State(state): State<SharedState>,
    body: Result<Json<JsonRpcRequest>, axum::extract::rejection::JsonRejection>,
) -> impl IntoResponse {
    // Parse JSON-RPC Request (POST)
    let req = match body {
        Ok(Json(r)) => r,
        Err(e) => {
            eprintln!("JSON Parse Error: {}", e.body_text());
            return (
                StatusCode::BAD_REQUEST,
                Json(rpc_error(Value::Null, -32700, "Parse error")),
            )
                .into_response();
        }
    };

    let id = req.id.unwrap_or(Value::Null);
    let method_name = req.method.as_str();
    let params = req.params.unwrap_or(Value::Null);

    println!("MCP Call: {} (id: {:?})", method_name, id);

    // Dispatch Method
    let response_body = match method_name {
        "initialize" => rpc_success(id, handle_initialize()),
        "notifications/initialized" => rpc_success(id, json!({})),
        "tools/list" => rpc_success(id, handle_tools_list()),
        "resources/list" => rpc_success(id, handle_resources_list()),
        "resources/read" => rpc_success(id, handle_resources_read(&state).await),
        "tools/call" => {
            let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(Value::Null);

            match handle_tool_call(&state, tool_name, args) {
                Ok(result) => rpc_success(id, result),
                Err(msg) => rpc_error(id, -32602, msg), // Invalid params or internal error
            }
        }
        "ping" => rpc_success(id, json!({})), // Optional but good for health checks
        _ => {
            eprintln!("Unknown method: {}", method_name);
            rpc_error(id, -32601, "Method not found")
        }
    };

    Json(response_body).into_response()
}

// =============================================================================
// MCP Method Handlers
// =============================================================================

/// Handles `initialize` request (Handshake).
fn handle_initialize() -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": {
            "tools": { "listChanged": true },
            "resources": { "listChanged": true, "subscribe": true }
        },
        "serverInfo": {
            "name": SERVER_NAME,
            "version": "0.1.0"
        }
    })
}

/// Handles `tools/list` request.
fn handle_tools_list() -> Value {
    json!({
        "tools": [
            {
                "name": TOOL_NAME,
                "title": "Add items to cart",
                "description": "Adds the provided items to the active cart and returns its state.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "items": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "required": ["name"],
                                "properties": {
                                    "name": { "type": "string" },
                                    "quantity": { "type": "integer", "default": 1 }
                                },
                                "additionalProperties": true
                            }
                        },
                        "cartId": { "type": "string" }
                    },
                    "required": ["items"],
                    "additionalProperties": false
                },
                "_meta": widget_meta(None)
            },
            {
                "name": CHECKOUT_TOOL_NAME,
                "title": "Checkout",
                "description": "Checks out the current cart, clearing it and returning a receipt.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "cartId": { "type": "string" }
                    },
                    "additionalProperties": false
                },
                "_meta": widget_meta(None)
            }
        ],
        "_meta": widget_meta(None)
    })
}

/// Handles `resources/list` request.
fn handle_resources_list() -> Value {
    json!({
        "resources": [{
            "name": "Start shopping cart",
            "uri": WIDGET_TEMPLATE_URI,
            "mimeType": WIDGET_MIME_TYPE,
            "_meta": widget_meta(None)
        }],
        "_meta": widget_meta(None)
    })
}

/// Handles `resources/read` request.
async fn handle_resources_read(state: &AppState) -> Value {
    let html = state.load_widget_html().await.unwrap_or_default();
    json!({
        "contents": [{
            "uri": WIDGET_TEMPLATE_URI,
            "mimeType": WIDGET_MIME_TYPE,
            "text": html,
            "_meta": widget_meta(None)
        }],
        "_meta": widget_meta(None)
    })
}

/// Handles `tools/call` request (Business Logic).
pub fn handle_tool_call(state: &AppState, name: &str, args: Value) -> Result<Value, String> {
    match name {
        TOOL_NAME => handle_add_to_cart_tool(state, args),
        CHECKOUT_TOOL_NAME => handle_checkout_tool(state, args),
        _ => Err(format!("Unknown tool: {}", name)),
    }
}

/// Handles the add_to_cart tool functionality
fn handle_add_to_cart_tool(state: &AppState, args: Value) -> Result<Value, String> {
    let input: AddToCartInput =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let cart_id = get_or_create_cart_id(input.cart_id);

    // Update or initialize cart
    let mut cart_items = state.carts.entry(cart_id.clone()).or_default();

    // Update cart contents
    update_cart_with_new_items(&mut cart_items, input.items);

    let current_items = cart_items.clone();
    let message = format!("Cart {} now has {} item(s).", cart_id, current_items.len());

    Ok(json!({
        "content": [{ "type": "text", "text": message }],
        "structuredContent": {
            "cartId": cart_id,
            "items": current_items
        },
        "_meta": widget_meta(Some(&cart_id))
    }))
}

/// Handles the checkout tool functionality
fn handle_checkout_tool(state: &AppState, args: Value) -> Result<Value, String> {
    let input: CheckoutInput =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let cart_id = get_or_create_cart_id(input.cart_id);

    // Remove the cart from the state to clear it
    if let Some((_, items)) = state.carts.remove(&cart_id) {
        let item_summary = format_item_summary(&items);
        let message = format!("Checked out now: {}", item_summary);
        println!("BACKEND CHECKOUT: {}", message);

        Ok(json!({
            "content": [{ "type": "text", "text": message }],
            "structuredContent": {
                "cartId": cart_id,
                "items": [],
                "checkout": true
            },
            "_meta": widget_meta(Some(&cart_id))
        }))
    } else {
        // Handle empty cart case
        Ok(json!({
            "content": [{ "type": "text", "text": "Cart is empty." }],
            "structuredContent": {
                "cartId": cart_id,
                "items": [],
                "checkout": true
            },
            "_meta": widget_meta(Some(&cart_id))
        }))
    }
}
