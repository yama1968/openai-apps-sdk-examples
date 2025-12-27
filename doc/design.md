openai-apps-sdk-examples/doc/design.md
# Design: Frontend-Backend State Synchronization

## Objective
Enable the shopping cart frontend widget to propagate user changes (adding items, adjusting quantities) back to the Python backend server. This ensures the backend's in-memory state matches the user's visual state, allowing the LLM to see items added manually by the user.

## Scope
*   **Frontend**: `src/shopping-cart/index.tsx`
*   **Backend**: `shopping_cart_python/main.py`

## Architecture Changes

### 1. Backend (`shopping_cart_python/main.py`)

The backend currently communicates via the MCP protocol but exposes a FastAPI/Starlette app for serving resources. We will add a dedicated HTTP endpoint to receive state updates from the widget.

**Changes:**
*   Import `Body` or leverage Pydantic models for request validation.
*   Define a new route `POST /sync_cart` attached to the `app` instance.
*   The endpoint will accept a JSON payload containing `cartId` and a list of `items`.
*   **State Replacement Logic**: It will completely replace the current content of the cart in the global `carts` dictionary with the provided items. This ensures that any deletions or quantity updates (increases/decreases) made in the widget are reflected exactly in the backend.

```python
# Proposed addition to main.py
@app.post("/sync_cart")
async def sync_cart(payload: AddToCartInput):
    # Ensure the cart exists or use the provided ID
    cart_id = payload.cart_id or uuid4().hex
    # Overwrite the backend state with the frontend's definitive state
    carts[cart_id] = [_serialize_item(item) for item in payload.items]
    return {"status": "updated", "cartId": cart_id}
```

### 2. Frontend (`src/shopping-cart/index.tsx`)

The frontend manages local state but is isolated. We will implement a "sync" side effect that pushes the local state to the backend whenever it changes via user interaction.

**Changes:**
*   Define a `BACKEND_URL` constant (default: `http://localhost:8000`).
*   Implement a helper function `syncToBackend(cartId, items)` that performs a `fetch` POST request to `/sync_cart`.
*   Implement a simple ID generator (e.g., `crypto.randomUUID()` or a fallback) to ensure a `cartId` exists if the user starts shopping before the LLM does.
*   Modify `addItem` and `adjustQuantity` (handles +/-) functions to:
    1.  Calculate the new list of items (including updated quantities or removals).
    2.  Check for or generate a `cartId`.
    3.  Update the local widget state (existing behavior).
    4.  Call `syncToBackend` with the full updated items list and `cartId`.

**Data Flow:**
1.  User clicks "+" or "-" on an item in the widget.
2.  Frontend updates `window.openai.widgetState` via `setCartState`.
3.  Frontend asynchronously POSTs the new item list and `cartId` to `http://localhost:8000/sync_cart`.
4.  Backend updates its in-memory `carts` store.
5.  If the user subsequently asks the LLM "What is in my cart?", the tool call will read the updated list from `carts`.

## Implementation Plan

1.  **Modify Backend**: Add the `/sync_cart` endpoint to `main.py` utilizing the existing `AddToCartInput` model for validation.
2.  **Modify Frontend**: Update `index.tsx` to handle `cartId` generation and perform the API call in the state update handlers.
