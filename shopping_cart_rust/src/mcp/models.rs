//! MCP Protocol Models and Constants
//!
//! This module contains all data structures and constants related to the
//! Model Context Protocol (MCP) specification.

use serde::Deserialize;
use serde_json::Value;

// =============================================================================
// MCP Constants
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
// MCP Protocol Models
// =============================================================================

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
