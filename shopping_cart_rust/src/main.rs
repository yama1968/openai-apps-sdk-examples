use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

// --- Models ---

fn default_quantity() -> u32 {
    1
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct CartItem {
    pub name: String,
    #[serde(default = "default_quantity")]
    pub quantity: u32,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AddToCartInput {
    pub items: Vec<CartItem>,
    pub cart_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SyncResponse {
    status: String,
    #[serde(rename = "cartId")]
    cart_id: String,
}

// --- App State ---

type SharedState = Arc<AppState>;

struct AppState {
    carts: DashMap<String, Vec<CartItem>>,
    assets_dir: PathBuf,
}

// --- Constants ---

const TOOL_NAME: &str = "add_to_cart";
const WIDGET_TEMPLATE_URI: &str = "ui://widget/shopping-cart.html";
const MIME_TYPE: &str = "text/html+skybridge";

// --- Handlers ---

async fn load_widget_html(state: &AppState) -> Result<String, StatusCode> {
    let html_path = state.assets_dir.join("shopping-cart.html");
    if html_path.exists() {
        return tokio::fs::read_to_string(html_path)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Fallback logic
    if let Ok(mut entries) = tokio::fs::read_dir(&state.assets_dir).await {
        let mut fallbacks = Vec::new();
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("shopping-cart-") && name.ends_with(".html") {
                    fallbacks.push(path);
                }
            }
        }
        fallbacks.sort();
        if let Some(last) = fallbacks.last() {
            return tokio::fs::read_to_string(last)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    Err(StatusCode::NOT_FOUND)
}

fn widget_meta(session_id: Option<&str>) -> Value {
    let mut meta = json!({
        "openai/outputTemplate": WIDGET_TEMPLATE_URI,
        "openai/toolInvocation/invoking": "Preparing shopping cart",
        "openai/toolInvocation/invoked": "Shopping cart ready",
        "openai/widgetAccessible": true,
    });

    if let Some(sid) = session_id {
        meta.as_object_mut()
            .unwrap()
            .insert("openai/widgetSessionId".to_string(), json!(sid));
    }

    meta
}

/// Handler for frontend synchronization
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

/// Generic MCP handler (Simplified for this example)
async fn handle_mcp(
    State(state): State<SharedState>,
    method: axum::http::Method,
    request: Result<Json<Value>, axum::extract::rejection::JsonRejection>,
) -> impl IntoResponse {
    if method == axum::http::Method::GET {
        return (
            [("content-type", "text/event-stream")],
            "event: endpoint\ndata: /mcp\n\n",
        )
            .into_response();
    }

    let Json(request) = match request {
        Ok(j) => j,
        Err(rejection) => {
            eprintln!("MCP JSON Rejection: {}", rejection.body_text());
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "jsonrpc": "2.0",
                    "error": { "code": -32700, "message": rejection.body_text() }
                })),
            )
                .into_response();
        }
    };

    let method_name = request.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let id = request.get("id").cloned().unwrap_or(Value::Null);

    println!("MCP Request: {} (id: {:?})", method_name, id);

    let result: Result<Value, Value> = match method_name {
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {
                    "listChanged": true
                },
                "resources": {
                    "listChanged": true,
                    "subscribe": true
                }
            },
            "serverInfo": {
                "name": "shopping-cart-rust",
                "version": "0.1.0"
            }
        })),
        "notifications/initialized" => Ok(json!({})),
        "tools/list" => {
            Ok(json!({
                "tools": [{
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
                                    }
                                }
                            },
                            "cartId": { "type": "string" }
                        },
                        "required": ["items"]
                    },
                    "_meta": widget_meta(None)
                }],
                "_meta": widget_meta(None)
            }))
        }
        "resources/list" => {
            Ok(json!({
                "resources": [{
                    "name": "Start shopping cart",
                    "uri": WIDGET_TEMPLATE_URI,
                    "mimeType": MIME_TYPE,
                    "_meta": widget_meta(None)
                }],
                "_meta": widget_meta(None)
            }))
        }
        "resources/read" => {
            let html = load_widget_html(&state).await.unwrap_or_default();
            Ok(json!({
                "contents": [{
                    "uri": WIDGET_TEMPLATE_URI,
                    "mimeType": MIME_TYPE,
                    "text": html,
                    "_meta": widget_meta(None)
                }],
                "_meta": widget_meta(None)
            }))
        }
        "tools/call" => {
            let params = request.get("params").unwrap_or(&Value::Null);
            let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");

            if name == TOOL_NAME {
                let args = params.get("arguments").unwrap_or(&Value::Null);
                let input: AddToCartInput = match serde_json::from_value(args.clone()) {
                    Ok(i) => i,
                    Err(e) => {
                        eprintln!("Error parsing add_to_cart arguments: {} | Args: {:?}", e, args);
                        return Json(json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "error": { "code": -32602, "message": format!("Invalid params: {}", e) }
                        })).into_response();
                    }
                };

                let cart_id = input.cart_id.unwrap_or_else(|| Uuid::new_v4().simple().to_string());

                let mut cart_items = state.carts.entry(cart_id.clone()).or_insert(Vec::new());

                for incoming in input.items {
                    if let Some(existing) = cart_items.iter_mut().find(|i| i.name == incoming.name) {
                        existing.quantity += incoming.quantity;
                    } else {
                        cart_items.push(incoming);
                    }
                }

                let items_copy = cart_items.clone();
                let message = format!("Cart {} now has {} item(s).", cart_id, items_copy.len());

                Ok(json!({
                    "content": [{ "type": "text", "text": message }],
                    "structuredContent": {
                        "cartId": cart_id,
                        "items": items_copy
                    },
                    "_meta": widget_meta(None)
                }))
            } else {
                Ok(json!({ "isError": true, "content": [{ "type": "text", "text": "Unknown tool" }] }))
            }
        }
        _ => Err(json!({ "code": -32601, "message": "Method not found" })),
    };

    let response_body = match result {
        Ok(res) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": res
        }),
        Err(err) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": err
        }),
    };

    println!("MCP Response: {}", method_name);
    let response = Json(response_body).into_response();
    if !response.status().is_success() {
        eprintln!("MCP Response Error: Status {}", response.status());
    }
    response
}

// --- Main ---

#[tokio::main]
async fn main() {
    let current_dir = std::env::current_dir().unwrap();
    let assets_dir = if current_dir.join("assets").exists() {
        current_dir.join("assets")
    } else if current_dir.parent().map(|p| p.join("assets").exists()).unwrap_or(false) {
        current_dir.parent().unwrap().join("assets")
    } else {
        // Fallback to searching relative to the executable or common project structure
        PathBuf::from("../assets")
    };

    println!("Using assets directory: {:?}", assets_dir);

    let state = Arc::new(AppState {
        carts: DashMap::new(),
        assets_dir,
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let mcp_router = post(handle_mcp).get(handle_mcp).options(handle_mcp);

    let app = Router::new()
        .route("/", mcp_router.clone())
        .route("/mcp", mcp_router.clone())
        .route("/mcp/", mcp_router)
        .route("/sync_cart", post(sync_cart).options(sync_cart))
        .layer(axum::middleware::from_fn(|req: Request<Body>, next: Next| async move {
            println!("Request: {} {} {:?}", req.method(), req.uri(), req.headers());
            let response = next.run(req).await;
            println!("Response Status: {}", response.status());
            response
        }))
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    println!("Rust Shopping Cart Server listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cart_logic() {
        let state = AppState {
            carts: DashMap::new(),
            assets_dir: PathBuf::from("."),
        };

        let cart_id = "test_cart".to_string();
        let item = CartItem {
            name: "Apple".to_string(),
            quantity: 2,
            extra: HashMap::new(),
        };

        // Initial add
        state.carts.insert(cart_id.clone(), vec![item.clone()]);

        // Aggregate add
        let mut items = state.carts.get_mut(&cart_id).unwrap();
        if let Some(existing) = items.iter_mut().find(|i| i.name == "Apple") {
            existing.quantity += 3;
        }

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].quantity, 5);
    }

    #[test]
    fn test_serialization() {
        let json_data = r#"{"items":[{"name":"Banana","quantity":1}],"cartId":"123"}"#;
        let input: AddToCartInput = serde_json::from_str(json_data).unwrap();
        assert_eq!(input.cart_id.unwrap(), "123");
        assert_eq!(input.items[0].name, "Banana");
    }
}
