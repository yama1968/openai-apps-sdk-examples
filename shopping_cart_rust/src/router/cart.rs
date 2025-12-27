//! Cart-related route handlers

use crate::model::{
    format_item_summary, get_or_create_cart_id, AddToCartInput, CheckoutInput, SharedState,
    SyncResponse,
};
use axum::{extract::State, response::IntoResponse, routing::post, Json, Router};

/// Creates routes for cart-related operations
pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/sync_cart", post(sync_cart))
        .route("/checkout", post(checkout))
}

/// Endpoint: POST /sync_cart
/// Updates the backend state to match the frontend (Widget) state exactly.
async fn sync_cart(
    State(state): State<SharedState>,
    Json(payload): Json<AddToCartInput>,
) -> impl IntoResponse {
    let cart_id = get_or_create_cart_id(payload.cart_id);

    state.carts.insert(cart_id.clone(), payload.items);

    Json(SyncResponse {
        status: "updated".to_string(),
        cart_id,
    })
}

/// Endpoint: POST /checkout
/// Processes checkout from the cart
async fn checkout(
    State(state): State<SharedState>,
    Json(payload): Json<CheckoutInput>,
) -> impl IntoResponse {
    let cart_id = get_or_create_cart_id(payload.cart_id);

    if let Some((_, items)) = state.carts.remove(&cart_id) {
        let item_summary = format_item_summary(&items);
        println!("REST API CHECKOUT: Cart {} - {}", cart_id, item_summary);
    }

    Json(SyncResponse {
        status: "checked_out".to_string(),
        cart_id,
    })
}
