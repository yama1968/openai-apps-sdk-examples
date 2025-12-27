use shopping_cart_rust::cart::AppState;
use shopping_cart_rust::router::create_app_router;
use std::net::SocketAddr;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // Initialize application state
    let state = Arc::new(AppState::new());

    // Build application router with all routes and middleware
    let app = create_app_router(state);

    // Configure the server address
    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    println!("Server running on http://{}", addr);

    // Start the server
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use shopping_cart_rust::cart::models::CartItem;
    use shopping_cart_rust::cart::state::AppState;
    use shopping_cart_rust::mcp::handlers::handle_tool_call;
    use shopping_cart_rust::mcp::models::TOOL_NAME;
    use std::collections::HashMap;

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

        // Use the handler from the public MCP module
        handle_tool_call(&state, TOOL_NAME, args, "default-test").expect("Tool call failed");

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
        use shopping_cart_rust::mcp::helpers::{rpc_error, rpc_success};
        let success = rpc_success(json!(1), json!("ok"));
        assert_eq!(success["result"], "ok");
        assert_eq!(success["id"], 1);

        let error = rpc_error(json!(2), -1, "fail");
        assert_eq!(error["error"]["message"], "fail");
        assert_eq!(error["id"], 2);
    }
}
