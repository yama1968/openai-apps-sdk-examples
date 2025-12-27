//! Routing module for the shopping cart application

use crate::cart::state::SharedState;
use axum::{body::Body, extract::Request, middleware::Next, Router};
use tower_http::cors::CorsLayer;

/// Creates and configures the application router with all routes and middleware
pub fn create_app_router(state: SharedState) -> Router {
    // Middleware: Log requests
    let log_layer = axum::middleware::from_fn(|req: Request<Body>, next: Next| async move {
        println!("REQ: {} {}", req.method(), req.uri());
        let res = next.run(req).await;
        if !res.status().is_success() {
            println!("RES: {} (Error)", res.status());
        }
        res
    });

    // Middleware: CORS (Permissive for local dev, allowing credentials)
    let cors_layer = CorsLayer::new()
        .allow_origin(tower_http::cors::AllowOrigin::mirror_request())
        .allow_credentials(true)
        .allow_methods(tower_http::cors::AllowMethods::mirror_request())
        .allow_headers(tower_http::cors::AllowHeaders::mirror_request());

    // Routes
    Router::new()
        .merge(crate::mcp::routes())
        .merge(crate::cart::routes())
        .layer(log_layer)
        .layer(cors_layer)
        .with_state(state)
}
