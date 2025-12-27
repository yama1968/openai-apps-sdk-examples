use axum::{
    body::Body,
    extract::{Request, State},
    http::{Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, net::SocketAddr, path::PathBuf, sync::Arc};
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

// -----------------------------------------------------------------------------
// Constants
// -----------------------------------------------------------------------------

const TOOL_NAME: &str = "add_to_cart";
const CHECKOUT_TOOL_NAME: &str = "checkout";
const WIDGET_TEMPLATE_URI: &str = "ui://widget/shopping-cart.html";
const WIDGET_MIME_TYPE: &str = "text/html+skybridge";
const SERVER_NAME: &str = "shopping-cart-rust";
const PROTOCOL_VERSION: &str = "2024-11-05";

// -----------------------------------------------------------------------------
// Data Models
// -----------------------------------------------------------------------------

fn default_quantity() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CartItem {
    pub name: String,
    #[serde(default = "default_quantity")]
    pub quantity: u32,
    /// Capture any extra fields (e.g., price, description) dynamically
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddToCartInput {
    pub items: Vec<CartItem>,
    pub cart_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CheckoutInput {
    #[serde(rename = "cartId")]
    pub cart_id: Option<String>,
}

#[derive(Serialize)]
struct SyncResponse {
    status: String,
    #[serde(rename = "cartId")]
    cart_id: String,
}

/// Standard JSON-RPC 2.0 Request envelope
#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: Option<String>,
    method: String,
    params: Option<Value>,
    id: Option<Value>,
}

// -----------------------------------------------------------------------------
// Application State
// -----------------------------------------------------------------------------

type SharedState = Arc<AppState>;

struct AppState {
    /// In-memory storage for carts, keyed by cart_id.
    /// DashMap allows concurrent access without external Mutexes.
    carts: DashMap<String, Vec<CartItem>>,
    /// Path to the directory containing HTML assets.
    assets_dir: PathBuf,
}

impl AppState {
    fn new() -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        // Strategy to locate assets:
        // 1. ./assets
        // 2. ../assets (if running from a subdir)
        let assets_dir = if current_dir.join("assets").exists() {
            current_dir.join("assets")
        } else if let Some(parent) = current_dir.parent() {
            if parent.join("assets").exists() {
                parent.join("assets")
            } else {
                PathBuf::from("assets") // Fallback
            }
        } else {
            PathBuf::from("assets")
        };

        println!("Using assets directory: {:?}", assets_dir);

        Self {
            carts: DashMap::new(),
            assets_dir,
        }
    }

    /// Reads the shopping-cart.html file or a fallback.
    async fn load_widget_html(&self) -> Result<String, StatusCode> {
        let html_path = self.assets_dir.join("shopping-cart.html");
        if html_path.exists() {
            return tokio::fs::read_to_string(html_path)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
        }

        // Search for fallbacks (e.g., shopping-cart-123.html)
        let mut entries = tokio::fs::read_dir(&self.assets_dir)
            .await
            .map_err(|_| StatusCode::NOT_FOUND)?;

        let mut fallbacks = Vec::new();
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("shopping-cart-") && name.ends_with(".html") {
                    fallbacks.push(path);
                }
            }
        }

        // Use the lexicographically last fallback (likely the latest build)
        fallbacks.sort();
        if let Some(last) = fallbacks.last() {
            tokio::fs::read_to_string(last)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        } else {
            Err(StatusCode::NOT_FOUND)
        }
    }
}

// -----------------------------------------------------------------------------
// Helper Functions
// -----------------------------------------------------------------------------

/// Construct the standard metadata required by the OpenAI widget system.
fn widget_meta() -> Value {
    json!({
        "openai/outputTemplate": WIDGET_TEMPLATE_URI,
        "openai/toolInvocation/invoking": "Preparing shopping cart",
        "openai/toolInvocation/invoked": "Shopping cart ready",
        "openai/widgetAccessible": true,
    })
}

/// Wraps a successful result in a JSON-RPC 2.0 Success Response.
fn rpc_success(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
}

/// Wraps an error in a JSON-RPC 2.0 Error Response.
fn rpc_error(id: Value, code: i32, message: impl Into<String>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message.into()
        }
    })
}

// -----------------------------------------------------------------------------
// MCP Method Handlers
// -----------------------------------------------------------------------------

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
                "_meta": widget_meta()
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
                "_meta": widget_meta()
            }
        ],
        "_meta": widget_meta()
    })
}

/// Handles `resources/list` request.
fn handle_resources_list() -> Value {
    json!({
        "resources": [{
            "name": "Start shopping cart",
            "uri": WIDGET_TEMPLATE_URI,
            "mimeType": WIDGET_MIME_TYPE,
            "_meta": widget_meta()
        }],
        "_meta": widget_meta()
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
            "_meta": widget_meta()
        }],
        "_meta": widget_meta()
    })
}

/// Handles `tools/call` request (Business Logic).
fn handle_tool_call(state: &AppState, name: &str, args: Value) -> Result<Value, String> {
    if name == TOOL_NAME {
        let input: AddToCartInput =
            serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

        let cart_id = input
            .cart_id
            .unwrap_or_else(|| Uuid::new_v4().simple().to_string());

        // Logic: Aggregate quantities if item exists, otherwise append
        let mut cart_items = state.carts.entry(cart_id.clone()).or_insert(Vec::new());

        for incoming in input.items {
            if let Some(existing) = cart_items.iter_mut().find(|i| i.name == incoming.name) {
                existing.quantity += incoming.quantity;
                // Merge extra fields if needed, or overwrite? Python version doesn't merge extra, it just updates qty.
            } else {
                cart_items.push(incoming);
            }
        }

        let current_items = cart_items.clone();
        let message = format!("Cart {} now has {} item(s).", cart_id, current_items.len());

        Ok(json!({
            "content": [{ "type": "text", "text": message }],
            "structuredContent": {
                "cartId": cart_id,
                "items": current_items
            },
            "_meta": widget_meta()
        }))
    } else if name == CHECKOUT_TOOL_NAME {
        let input: CheckoutInput =
            serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

        let cart_id = input
            .cart_id
            .unwrap_or_else(|| Uuid::new_v4().simple().to_string());

        // Remove the cart from the state to clear it
        if let Some((_, items)) = state.carts.remove(&cart_id) {
            let item_summary = items
                .iter()
                .map(|i| format!("{}x {}", i.quantity, i.name))
                .collect::<Vec<_>>()
                .join(", ");

            let message = format!("Checked out now: {}", item_summary);
            println!("BACKEND CHECKOUT: {}", message);

            Ok(json!({
                "content": [{ "type": "text", "text": message }],
                "structuredContent": {
                    "cartId": cart_id,
                    "items": [],
                    "checkout": true
                },
                "_meta": widget_meta()
            }))
        } else {
            Ok(json!({
                "content": [{ "type": "text", "text": "Cart is empty." }],
                "structuredContent": {
                    "cartId": cart_id,
                    "items": [],
                    "checkout": true
                },
                "_meta": widget_meta()
            }))
        }
    } else {
        Err(format!("Unknown tool: {}", name))
    }
}

// -----------------------------------------------------------------------------
// HTTP Handlers
// -----------------------------------------------------------------------------

/// Endpoint: POST /sync_cart
/// Updates the backend state to match the frontend (Widget) state exactly.
async fn sync_cart(
    State(state): State<SharedState>,
    Json(payload): Json<AddToCartInput>,
) -> impl IntoResponse {
    let cart_id = payload
        .cart_id
        .unwrap_or_else(|| Uuid::new_v4().simple().to_string());

    state.carts.insert(cart_id.clone(), payload.items);

    Json(SyncResponse {
        status: "updated".to_string(),
        cart_id,
    })
}

async fn checkout(
    State(state): State<SharedState>,
    Json(payload): Json<CheckoutInput>,
) -> impl IntoResponse {
    let cart_id = payload
        .cart_id
        .unwrap_or_else(|| Uuid::new_v4().simple().to_string());

    if let Some((_, items)) = state.carts.remove(&cart_id) {
        let item_summary = items
            .iter()
            .map(|i| format!("{}x {}", i.quantity, i.name))
            .collect::<Vec<_>>()
            .join(", ");
        println!("REST API CHECKOUT: Cart {} - {}", cart_id, item_summary);
    }

    Json(SyncResponse {
        status: "checked_out".to_string(),
        cart_id,
    })
}

/// Endpoint: GET/POST /mcp
/// Handles the Model Context Protocol communication.
async fn handle_mcp(
    State(state): State<SharedState>,
    method: Method,
    body: Result<Json<JsonRpcRequest>, axum::extract::rejection::JsonRejection>,
) -> impl IntoResponse {
    // 1. Handle SSE Handshake (GET)
    if method == Method::GET {
        return (
            [("content-type", "text/event-stream")],
            "event: endpoint\ndata: /mcp\n\n",
        )
            .into_response();
    }

    // 2. Parse JSON-RPC Request (POST)
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

    // 3. Dispatch Method
    let response_body = match method_name {
        "initialize" => rpc_success(id, handle_initialize()),
        "notifications/initialized" => rpc_success(id, json!({})),

        "tools/list" => rpc_success(id, handle_tools_list()),

        "resources/list" => rpc_success(id, handle_resources_list()),

        "resources/read" => rpc_success(id, handle_resources_read(&state).await),

        "tools/call" => {
            let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(Value::Null);

            match handle_tool_call(&state, name, args) {
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

// -----------------------------------------------------------------------------
// Main Entrypoint
// -----------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState::new());

    // Middleware: Log requests
    let log_layer = axum::middleware::from_fn(|req: Request<Body>, next: Next| async move {
        println!("REQ: {} {}", req.method(), req.uri());
        let res = next.run(req).await;
        if !res.status().is_success() {
            println!("RES: {} (Error)", res.status());
        }
        res
    });

    // Middleware: CORS (Permissive for local dev)
    let cors_layer = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Routes
    let app = Router::new()
        .route("/", post(handle_mcp).get(handle_mcp))
        .route("/mcp", post(handle_mcp).get(handle_mcp)) // Standard endpoint
        .route("/mcp/", post(handle_mcp).get(handle_mcp)) // Trailing slash safety
        .route("/sync_cart", post(sync_cart).options(sync_cart))
        .route("/checkout", post(checkout).options(checkout))
        .layer(log_layer)
        .layer(cors_layer)
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    println!("Server running on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_state_manager_and_aggregation() {
        let state = AppState::new();
        let cart_id = "test_cart_1";

        // 1. Initial Insert (Simulate Sync)
        let initial_items = vec![CartItem {
            name: "Apple".into(),
            quantity: 2,
            extra: HashMap::new(),
        }];
        state.carts.insert(cart_id.into(), initial_items);

        // 2. Tool Call (Simulate Add)
        let args = json!({
            "cartId": cart_id,
            "items": [
                { "name": "Apple", "quantity": 3 },
                { "name": "Banana", "quantity": 1 }
            ]
        });

        handle_tool_call(&state, TOOL_NAME, args).expect("Tool call failed");

        // 3. Verify
        let items = state.carts.get(cart_id).unwrap();

        let apple = items.iter().find(|i| i.name == "Apple").unwrap();
        assert_eq!(
            apple.quantity, 5,
            "Apple quantity should aggregate to 2+3=5"
        );

        let banana = items.iter().find(|i| i.name == "Banana").unwrap();
        assert_eq!(banana.quantity, 1, "Banana should be added");
    }

    #[test]
    fn test_rpc_envelopes() {
        let success = rpc_success(json!(1), json!("ok"));
        assert_eq!(success["result"], "ok");
        assert_eq!(success["id"], 1);

        let error = rpc_error(json!(2), -1, "fail");
        assert_eq!(error["error"]["message"], "fail");
        assert_eq!(error["id"], 2);
    }
}
