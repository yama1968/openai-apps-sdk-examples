# Design: Rust Port of Shopping Cart Server

## Objective
Rewrite the `shopping_cart_python` server in Rust, maintaining full compatibility with the existing React frontend and the MCP (Model Context Protocol). The server will serve both as an MCP server and a standard HTTP API for state synchronization.

## Target Location
`openai-apps-sdk-examples/shopping_cart_rust/`

## Proposed Technology Stack

### Core Frameworks
*   **[Axum](https://github.com/tokio-rs/axum)**: A web framework built on `tokio`, `tower`, and `hyper`. It is ideal for handling both the HTTP sync endpoint and the MCP transport.
*   **[Tokio](https://tokio.rs/)**: The industry-standard asynchronous runtime for Rust.
*   **[Serde](https://serde.rs/)**: For high-performance serialization and deserialization of JSON payloads.

### Utilities
*   **[Tower-HTTP](https://docs.rs/tower-http/latest/tower_http/)**: Specifically for the `CorsLayer` to allow frontend-to-backend communication.
*   **[UUID](https://docs.rs/uuid/latest/uuid/)**: For generating unique `cartId` values.
*   **[DashMap](https://docs.rs/dashmap/latest/dashmap/)** or **`Arc<RwLock<HashMap<...>>>`**: For managing concurrent access to the in-memory cart storage.

## Architecture

### 1. Data Models
Mirroring the Pydantic models in Python:
*   `CartItem`: `name` (String), `quantity` (u32), plus an "extra" fields map for metadata.
*   `AddToCartInput`: `items` (Vec<CartItem>), `cart_id` (Option<String>).
*   `CartState`: The internal storage structure mapping `String` IDs to `Vec<CartItem>`.

### 2. State Management
In Rust, the `carts` dictionary will be stored in an `Arc<RwLock<HashMap<String, Vec<CartItem>>>>` to allow safe, concurrent access across multiple async tasks (HTTP requests).

### 3. Endpoints & Routes

#### MCP Integration
The server will implement the MCP spec (likely over JSON-RPC via HTTP for consistency with the Python `streamable_http_app`).
*   `POST /` (or specific MCP path): Handles JSON-RPC requests for `tools/list`, `resources/list`, `tools/call` (`add_to_cart`), and `resources/read`.

#### Sync Endpoint
*   **`POST /sync_cart`**:
    *   Accepts `AddToCartInput`.
    *   Acquires a write lock on the state.
    *   Overwrites the cart's content with the provided items.
    *   Returns a JSON response with status and `cartId`.

### 4. Minimal Logic Implementation
*   **`add_to_cart` tool**: When called, it will search for existing items by name and increment quantities (matching the fix applied to the Python version).
*   **HTML Asset Serving**: The server will read the `shopping-cart.html` from the `assets/` directory (relative to the project root).

## Implementation Plan

### Phase 1: Project Setup
*   Initialize `Cargo.toml` with `axum`, `tokio`, `serde`, `serde_json`, `tower-http`, and `uuid`.
*   Define the directory structure (`src/main.rs`, `src/models.rs`, `src/handlers/`).

### Phase 2: Core Logic
*   Implement the shared state using `Arc` and `RwLock`.
*   Implement JSON-RPC request/response types for MCP.
*   Port the `_load_widget_html` logic to read from the filesystem.

### Phase 3: Web Server & Synchronization
*   Implement the Axum router.
*   Add CORS middleware to allow requests from the OpenAI platform.
*   Implement the `/sync_cart` handler.

### Phase 4: MCP Handlers
*   Implement `handle_call_tool` for `add_to_cart`.
*   Implement `handle_read_resource` for the widget HTML.

## Key Differences from Python
*   **Type Safety**: Use of Enums and Structs for MCP messages instead of generic dictionaries.
*   **Concurrency**: Explicit handling of thread safety for the global cart state.
*   **Performance**: Significant reduction in memory footprint and improved request throughput.
