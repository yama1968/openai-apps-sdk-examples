//! Integration tests for MCP (Model Context Protocol) server
//!
//! These tests verify the complete MCP protocol implementation including:
//! - Server initialization and handshake
//! - Tool discovery and listing
//! - Resource discovery and reading
//! - Tool execution (add_to_cart, checkout)
//! - Error handling

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::util::ServiceExt; // for `oneshot`

// Import from the main crate
use shopping_cart_rust::cart::AppState;
use shopping_cart_rust::router::create_app_router;

/// Helper function to create a test app instance
fn create_test_app() -> axum::Router {
    // Set a predictable assets directory for tests if needed,
    // but the default AppState::new() should handle it.
    let state = Arc::new(AppState::new());
    create_app_router(state)
}

/// Helper function to send a JSON request and get the response (REST API)
async fn send_rest_request(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Value,
) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&body_bytes).unwrap_or(json!({}));

    (status, body)
}

/// Helper function to send a JSON-RPC request and get the response
async fn send_jsonrpc_request(
    app: &axum::Router,
    method: &str,
    params: Option<Value>,
    id: i32,
) -> (StatusCode, Value) {
    let request_body = json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": id
    });

    let request = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&body_bytes).unwrap_or(json!({}));

    (status, body)
}

#[tokio::test]
async fn test_mcp_sse_endpoint() {
    let app = create_test_app();

    let request = Request::builder()
        .method("GET")
        .uri("/mcp")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let content_type = response
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(content_type, "text/event-stream");

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();

    assert!(body_str.contains("event: endpoint"));
    assert!(body_str.contains("data: /mcp"));
}

#[tokio::test]
async fn test_mcp_initialize() {
    let app = create_test_app();

    let (status, body) = send_jsonrpc_request(&app, "initialize", None, 1).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["jsonrpc"], "2.0");
    assert_eq!(body["id"], 1);

    let result = &body["result"];
    assert_eq!(result["protocolVersion"], "2024-11-05");
    assert_eq!(result["serverInfo"]["name"], "shopping-cart-rust");
    assert!(result["capabilities"]["tools"]["listChanged"]
        .as_bool()
        .unwrap());
    assert!(result["capabilities"]["resources"]["listChanged"]
        .as_bool()
        .unwrap());
}

#[tokio::test]
async fn test_mcp_tools_list() {
    let app = create_test_app();

    let (status, body) = send_jsonrpc_request(&app, "tools/list", None, 2).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["jsonrpc"], "2.0");
    assert_eq!(body["id"], 2);

    let tools = body["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 2);

    // Check add_to_cart tool
    let add_to_cart = &tools[0];
    assert_eq!(add_to_cart["name"], "add_to_cart");
    assert_eq!(add_to_cart["title"], "Add items to cart");
    assert!(!add_to_cart["description"].as_str().unwrap().is_empty());
    assert!(add_to_cart["inputSchema"]["properties"]["items"].is_object());

    // Check checkout tool
    let checkout = &tools[1];
    assert_eq!(checkout["name"], "checkout");
    assert_eq!(checkout["title"], "Checkout");
    assert!(checkout["inputSchema"]["properties"]["cartId"].is_object());
}

#[tokio::test]
async fn test_mcp_resources_list() {
    let app = create_test_app();

    let (status, body) = send_jsonrpc_request(&app, "resources/list", None, 3).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["jsonrpc"], "2.0");

    let resources = body["result"]["resources"].as_array().unwrap();
    assert_eq!(resources.len(), 1);

    let widget = &resources[0];
    assert_eq!(widget["name"], "Start shopping cart");
    assert_eq!(widget["uri"], "ui://widget/shopping-cart.html");
    assert_eq!(widget["mimeType"], "text/html+skybridge");
}

#[tokio::test]
async fn test_mcp_resources_read() {
    let app = create_test_app();

    let (status, body) = send_jsonrpc_request(&app, "resources/read", None, 4).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["jsonrpc"], "2.0");

    let contents = body["result"]["contents"].as_array().unwrap();
    assert_eq!(contents.len(), 1);

    let content = &contents[0];
    assert_eq!(content["uri"], "ui://widget/shopping-cart.html");
    assert_eq!(content["mimeType"], "text/html+skybridge");
    // HTML content might be empty or a fallback, but the field should exist
    assert!(content["text"].is_string());
}

#[tokio::test]
async fn test_mcp_tool_call_add_to_cart() {
    let app = create_test_app();

    let params = json!({
        "name": "add_to_cart",
        "arguments": {
            "items": [
                { "name": "Apple", "quantity": 3 },
                { "name": "Banana", "quantity": 2 }
            ]
        }
    });

    let (status, body) = send_jsonrpc_request(&app, "tools/call", Some(params), 5).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["jsonrpc"], "2.0");
    assert_eq!(body["id"], 5);

    let result = &body["result"];
    let content = &result["content"][0];
    assert_eq!(content["type"], "text");
    assert!(content["text"].as_str().unwrap().contains("now has 2 item"));

    let structured = &result["structuredContent"];
    assert!(structured["cartId"].is_string());

    let items = structured["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["name"], "Apple");
    assert_eq!(items[0]["quantity"], 3);
    assert_eq!(items[1]["name"], "Banana");
    assert_eq!(items[1]["quantity"], 2);
}

#[tokio::test]
async fn test_mcp_tool_call_add_to_cart_aggregation() {
    let app = create_test_app();

    // First call: add 2 apples
    let params1 = json!({
        "name": "add_to_cart",
        "arguments": {
            "cartId": "test-cart-123",
            "items": [{ "name": "Apple", "quantity": 2 }]
        }
    });

    let (status1, body1) = send_jsonrpc_request(&app, "tools/call", Some(params1), 6).await;
    assert_eq!(status1, StatusCode::OK);

    let cart_id = body1["result"]["structuredContent"]["cartId"]
        .as_str()
        .unwrap();
    assert_eq!(cart_id, "test-cart-123");

    // Second call: add 3 more apples to the same cart
    let params2 = json!({
        "name": "add_to_cart",
        "arguments": {
            "cartId": "test-cart-123",
            "items": [{ "name": "Apple", "quantity": 3 }]
        }
    });

    let (status2, body2) = send_jsonrpc_request(&app, "tools/call", Some(params2), 7).await;
    assert_eq!(status2, StatusCode::OK);

    // Verify aggregation: should have 5 apples total
    let items = body2["result"]["structuredContent"]["items"]
        .as_array()
        .unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["name"], "Apple");
    assert_eq!(items[0]["quantity"], 5);
}

#[tokio::test]
async fn test_mcp_tool_call_checkout() {
    let app = create_test_app();

    // First add items to cart
    let add_params = json!({
        "name": "add_to_cart",
        "arguments": {
            "cartId": "checkout-test-cart",
            "items": [
                { "name": "Apple", "quantity": 2 },
                { "name": "Banana", "quantity": 1 }
            ]
        }
    });

    let (_, add_body) = send_jsonrpc_request(&app, "tools/call", Some(add_params), 8).await;
    let cart_id = add_body["result"]["structuredContent"]["cartId"]
        .as_str()
        .unwrap();

    // Now checkout
    let checkout_params = json!({
        "name": "checkout",
        "arguments": {
            "cartId": cart_id
        }
    });

    let (status, body) = send_jsonrpc_request(&app, "tools/call", Some(checkout_params), 9).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["jsonrpc"], "2.0");

    let result = &body["result"];
    let content = &result["content"][0];
    assert!(content["text"].as_str().unwrap().contains("Checked out"));

    let structured = &result["structuredContent"];
    assert_eq!(structured["checkout"], true);
    assert_eq!(structured["items"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_mcp_tool_call_checkout_empty_cart() {
    let app = create_test_app();

    let params = json!({
        "name": "checkout",
        "arguments": {
            "cartId": "nonexistent-cart"
        }
    });

    let (status, body) = send_jsonrpc_request(&app, "tools/call", Some(params), 10).await;

    assert_eq!(status, StatusCode::OK);

    let result = &body["result"];
    let content = &result["content"][0];
    assert_eq!(content["text"], "Cart is empty.");
}

#[tokio::test]
async fn test_mcp_unknown_method() {
    let app = create_test_app();

    let (status, body) = send_jsonrpc_request(&app, "unknown/method", None, 11).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["jsonrpc"], "2.0");
    assert_eq!(body["id"], 11);

    let error = &body["error"];
    assert_eq!(error["code"], -32601);
    assert_eq!(error["message"], "Method not found");
}

#[tokio::test]
async fn test_mcp_invalid_json() {
    let app = create_test_app();

    let request = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .body(Body::from("invalid json {{{"))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(body["error"]["code"], -32700);
    assert_eq!(body["error"]["message"], "Parse error");
}

#[tokio::test]
async fn test_mcp_tool_call_unknown_tool() {
    let app = create_test_app();

    let params = json!({
        "name": "unknown_tool",
        "arguments": {}
    });

    let (status, body) = send_jsonrpc_request(&app, "tools/call", Some(params), 12).await;

    assert_eq!(status, StatusCode::OK);

    let error = &body["error"];
    assert_eq!(error["code"], -32602);
    assert!(error["message"].as_str().unwrap().contains("Unknown tool"));
}

#[tokio::test]
async fn test_mcp_tool_call_invalid_arguments() {
    let app = create_test_app();

    let params = json!({
        "name": "add_to_cart",
        "arguments": {
            "invalid_field": "value"
        }
    });

    let (status, body) = send_jsonrpc_request(&app, "tools/call", Some(params), 13).await;

    assert_eq!(status, StatusCode::OK);

    let error = &body["error"];
    assert_eq!(error["code"], -32602);
    assert!(error["message"]
        .as_str()
        .unwrap()
        .contains("Invalid arguments"));
}

#[tokio::test]
async fn test_mcp_ping() {
    let app = create_test_app();

    let (status, body) = send_jsonrpc_request(&app, "ping", None, 14).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["jsonrpc"], "2.0");
    assert_eq!(body["id"], 14);
    assert_eq!(body["result"], json!({}));
}

#[tokio::test]
async fn test_mcp_notifications_initialized() {
    let app = create_test_app();

    let (status, body) = send_jsonrpc_request(&app, "notifications/initialized", None, 15).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["jsonrpc"], "2.0");
    assert_eq!(body["result"], json!({}));
}

#[tokio::test]
async fn test_multiple_carts_isolation() {
    let app = create_test_app();

    // Create cart 1
    let params1 = json!({
        "name": "add_to_cart",
        "arguments": {
            "cartId": "cart-1",
            "items": [{ "name": "Apple", "quantity": 5 }]
        }
    });

    let (status1, _) = send_jsonrpc_request(&app, "tools/call", Some(params1), 16).await;
    assert_eq!(status1, StatusCode::OK);

    // Create cart 2
    let params2 = json!({
        "name": "add_to_cart",
        "arguments": {
            "cartId": "cart-2",
            "items": [{ "name": "Banana", "quantity": 3 }]
        }
    });

    let (status2, body2) = send_jsonrpc_request(&app, "tools/call", Some(params2), 17).await;
    assert_eq!(status2, StatusCode::OK);

    // Verify cart 2 only has bananas
    let items = body2["result"]["structuredContent"]["items"]
        .as_array()
        .unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["name"], "Banana");
    assert_eq!(items[0]["quantity"], 3);
}
#[tokio::test]
async fn test_rest_sync_cart() {
    let app = create_test_app();

    let payload = json!({
        "items": [
            { "name": "Apple", "quantity": 10 }
        ],
        "cartId": "rest-test-cart"
    });

    let (status, body) = send_rest_request(&app, "POST", "/sync_cart", payload).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "updated");
    assert_eq!(body["cartId"], "rest-test-cart");
}

#[tokio::test]
async fn test_rest_checkout() {
    let app = create_test_app();

    // First sync a cart
    let sync_payload = json!({
        "items": [{ "name": "Banana", "quantity": 5 }],
        "cartId": "checkout-rest-cart"
    });
    send_rest_request(&app, "POST", "/sync_cart", sync_payload).await;

    // Then checkout
    let checkout_payload = json!({
        "cartId": "checkout-rest-cart"
    });
    let (status, body) = send_rest_request(&app, "POST", "/checkout", checkout_payload).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "checked_out");
    assert_eq!(body["cartId"], "checkout-rest-cart");
}

#[tokio::test]
async fn test_rest_checkout_no_id() {
    let app = create_test_app();

    let (status, body) = send_rest_request(&app, "POST", "/checkout", json!({})).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "checked_out");
    assert!(body["cartId"].is_string());
}

#[tokio::test]
async fn test_mcp_invalid_method_type() {
    let app = create_test_app();

    // method should be a string, let's pass a number
    let request_body = json!({
        "jsonrpc": "2.0",
        "method": 123,
        "id": 1
    });

    let request = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    // Rejection by Axum Json extractor or our handler
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
