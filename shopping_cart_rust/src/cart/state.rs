//! Shopping Cart State Management
//!
//! This module manages the application state for shopping carts,
//! including cart storage and asset file management.

use super::models::CartItem;
use dashmap::DashMap;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

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

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
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
    fn locate_assets_directory(current_dir: &Path) -> PathBuf {
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
