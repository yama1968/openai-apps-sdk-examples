//! Model Context Protocol (MCP) Module
//!
//! This module contains all MCP protocol implementation, including:
//! - Protocol models (JsonRpcRequest, constants)
//! - RPC helpers (success/error responses, widget metadata)
//! - MCP handlers (initialize, tools/list, tools/call, etc.)

pub mod handlers;
pub mod helpers;
pub mod models;

// Re-export commonly used types and functions
pub use handlers::routes;
