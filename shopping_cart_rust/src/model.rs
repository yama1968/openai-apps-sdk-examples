pub use crate::helpers::{
    format_item_summary, get_or_create_cart_id, rpc_error, rpc_success, update_cart_with_new_items,
    widget_meta,
};
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
pub const WIDGET_TEMPLATE_URI: &str = "ui://widget/shopping-cart.html";
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

    /// Reads the shopping-cart.html file or a fallback version
    pub async fn load_widget_html(&self) -> Result<String, axum::http::StatusCode> {
        // First try the primary HTML file
        let primary_html_path = self.assets_dir.join("shopping-cart.html");
        if primary_html_path.exists() {
            return tokio::fs::read_to_string(primary_html_path)
                .await
                .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        }

        // Search for fallbacks (e.g., shopping-cart-123.html)
        let fallback_path = self.find_fallback_html_file().await?;

        tokio::fs::read_to_string(fallback_path)
            .await
            .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)
    }

    /// Finds a fallback HTML file when the primary one is not available
    async fn find_fallback_html_file(&self) -> Result<PathBuf, axum::http::StatusCode> {
        let mut entries = tokio::fs::read_dir(&self.assets_dir)
            .await
            .map_err(|_| axum::http::StatusCode::NOT_FOUND)?;

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
        fallbacks
            .last()
            .cloned()
            .ok_or(axum::http::StatusCode::NOT_FOUND)
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/* Helper utilities are re‑exported via `pub use helpers::{...}`; no additional code is needed here. */
