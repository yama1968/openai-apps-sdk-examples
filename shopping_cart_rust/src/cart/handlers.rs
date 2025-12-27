//! REST API handlers for shopping cart operations
//!
//! This module implements HTTP endpoints for cart synchronization
//! and checkout operations.

use super::{helpers::*, models::*, state::SharedState};
use axum::{extract::State, response::IntoResponse, routing::post, Json, Router};

/// Creates routes for cart-related operations
pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/sync_cart", post(sync_cart))
        .route("/checkout", post(checkout))
}

use axum::http::HeaderMap;

/// Endpoint: POST /sync_cart
/// Updates the backend state to match the frontend (Widget) state exactly.
async fn sync_cart(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(payload): Json<AddToCartInput>,
) -> impl IntoResponse {
    let (session_id, is_new_session) = resolve_session_id(&headers);
    let cart_id = get_or_default_cart_id(payload.cart_id, &session_id);

    state.carts.insert(cart_id.clone(), payload.items);

    let mut response = Json(SyncResponse {
        status: "updated".to_string(),
        cart_id,
    })
    .into_response();

    if is_new_session {
        let cookie_val = format!("cart_session={}; Path=/; HttpOnly", session_id);
        response
            .headers_mut()
            .insert(axum::http::header::SET_COOKIE, cookie_val.parse().unwrap());
    }

    response
}

/// Endpoint: POST /checkout
/// Processes checkout from the cart
async fn checkout(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(payload): Json<CheckoutInput>,
) -> impl IntoResponse {
    let (session_id, is_new_session) = resolve_session_id(&headers);
    let cart_id = get_or_default_cart_id(payload.cart_id, &session_id);

    if let Some((_, items)) = state.carts.remove(&cart_id) {
        let item_summary = format_item_summary(&items);
        println!("REST API CHECKOUT: Cart {} - {}", cart_id, item_summary);
    }

    let mut response = Json(SyncResponse {
        status: "checked_out".to_string(),
        cart_id,
    })
    .into_response();

    if is_new_session {
        let cookie_val = format!("cart_session={}; Path=/; HttpOnly", session_id);
        response
            .headers_mut()
            .insert(axum::http::header::SET_COOKIE, cookie_val.parse().unwrap());
    }

    response
}
