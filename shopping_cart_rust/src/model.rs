use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, path::PathBuf, sync::Arc};

// =============================================================================
// Constants
// =============================================================================

/// Name of the primary tool for adding items to a cart
pub const TOOL_NAME: &str = "add_to_cart";
/// Name of the checkout tool
pub const CHECKOUT_TOOL_NAME: &str = "checkout";
/// URI for the widget template
pub const WIDGET_TEMPLATE_URI: &str = "ui://widget/vanilla-shopping-cart.html";
/// MIME type for the widget
pub const WIDGET_MIME_TYPE: &str = "text/html+skybridge";
/// Server identifier
pub const SERVER_NAME: &str = "shopping-cart-rust";
/// Protocol version for MCP
pub const PROTOCOL_VERSION: &str = "2024-11-05";

// =============================================================================
// Data Models
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

/// Standard JSON-RPC 2.0 Request envelope
#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    /// Protocol version (should be "2.0")
    #[allow(dead_code)]
    pub jsonrpc: Option<String>,

    /// Method name to invoke
    pub method: String,

    /// Parameters for the method
    pub params: Option<Value>,

    /// Request identifier
    pub id: Option<Value>,
}

// =============================================================================
// Application State
// =============================================================================

/// Shared application state that can be safely passed between threads
pub type SharedState = Arc<AppState>;

/// Core application state containing carts and asset information
pub struct AppState {
    /// In-memory storage for carts, keyed by cart_id.
    /// DashMap allows concurrent access without external Mutexes.
    pub carts: DashMap<String, Vec<CartItem>>,

    /// Path to the directory containing HTML assets.
    pub assets_dir: PathBuf,
}

impl AppState {
    /// Creates a new AppState with empty carts and locates the assets directory
    pub fn new() -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let assets_dir = Self::locate_assets_directory(&current_dir);

        println!("Using assets directory: {:?}", assets_dir);

        Self {
            carts: DashMap::new(),
            assets_dir,
        }
    }

    /// Attempts to locate the assets directory using a multi-step strategy
    fn locate_assets_directory(current_dir: &PathBuf) -> PathBuf {
        // Strategy to locate assets:
        // 1. ./assets
        // 2. ../assets (if running from a subdir)
        // 3. Fallback to "assets" relative path

        if current_dir.join("assets").exists() {
            return current_dir.join("assets");
        }

        if let Some(parent) = current_dir.parent() {
            if parent.join("assets").exists() {
                return parent.join("assets");
            }
        }

        PathBuf::from("assets") // Fallback
    }

    /// Reads the widget HTML file or a fallback version
    pub async fn load_widget_html(&self) -> Result<String, axum::http::StatusCode> {
        // First try the vanilla version
        let vanilla_html_path = self.assets_dir.join("vanilla-shopping-cart.html");
        if vanilla_html_path.exists() {
            return tokio::fs::read_to_string(vanilla_html_path)
                .await
                .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        }

        // Then try the standard version
        let standard_html_path = self.assets_dir.join("shopping-cart.html");
        if standard_html_path.exists() {
            return tokio::fs::read_to_string(standard_html_path)
                .await
                .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        }

        // Search for fallbacks
        let fallback_path = self.find_fallback_html_file().await?;

        tokio::fs::read_to_string(fallback_path)
            .await
            .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)
    }

    /// Finds a fallback HTML file when the primary ones are not available
    async fn find_fallback_html_file(&self) -> Result<PathBuf, axum::http::StatusCode> {
        let mut entries = tokio::fs::read_dir(&self.assets_dir)
            .await
            .map_err(|_| axum::http::StatusCode::NOT_FOUND)?;

        let mut fallbacks = Vec::new();
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                // Look for both vanilla and standard fallbacks
                if (name.starts_with("vanilla-shopping-cart-")
                    || name.starts_with("shopping-cart-"))
                    && name.ends_with(".html")
                {
                    fallbacks.push(path);
                }
            }
        }

        // Sort the fallbacks to prioritize vanilla versions first, then by name
        fallbacks.sort_by(|a, b| {
            let a_name = a.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let b_name = b.file_name().and_then(|n| n.to_str()).unwrap_or("");

            let a_is_vanilla = a_name.starts_with("vanilla-");
            let b_is_vanilla = b_name.starts_with("vanilla-");

            // Compare vanilla status first (vanilla comes before non-vanilla)
            if a_is_vanilla && !b_is_vanilla {
                return std::cmp::Ordering::Less;
            } else if !a_is_vanilla && b_is_vanilla {
                return std::cmp::Ordering::Greater;
            }

            // If both have same vanilla status, sort by name (latest version last)
            b_name.cmp(a_name)
        });

        // Return the first fallback (highest priority based on our sort)
        fallbacks
            .first()
            .cloned()
            .ok_or(axum::http::StatusCode::NOT_FOUND)
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Construct the standard metadata required by the OpenAI widget system.
pub fn widget_meta() -> Value {
    json!({
        "openai/outputTemplate": WIDGET_TEMPLATE_URI,
        "openai/toolInvocation/invoking": "Preparing shopping cart",
        "openai/toolInvocation/invoked": "Shopping cart ready",
        "openai/widgetAccessible": true,
    })
}

/// Wraps a successful result in a JSON-RPC 2.0 Success Response.
pub fn rpc_success(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
}

/// Wraps an error in a JSON-RPC 2.0 Error Response.
pub fn rpc_error(id: Value, code: i32, message: impl Into<String>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message.into()
        }
    })
}

/// Generates a new cart ID if none is provided
pub fn get_or_create_cart_id(cart_id: Option<String>) -> String {
    cart_id.unwrap_or_else(|| uuid::Uuid::new_v4().simple().to_string())
}

/// Updates the cart with new items, aggregating quantities for existing items
pub fn update_cart_with_new_items(cart_items: &mut Vec<CartItem>, new_items: Vec<CartItem>) {
    for incoming in new_items {
        if let Some(existing) = cart_items.iter_mut().find(|i| i.name == incoming.name) {
            // Aggregate quantities for existing items
            existing.quantity += incoming.quantity;
            // Note: The Python version doesn't merge extra fields, it just updates quantity
        } else {
            // Add new items to the cart
            cart_items.push(incoming);
        }
    }
}

/// Formats items into a readable summary string
pub fn format_item_summary(items: &[CartItem]) -> String {
    items
        .iter()
        .map(|i| format!("{}x {}", i.quantity, i.name))
        .collect::<Vec<_>>()
        .join(", ")
}
