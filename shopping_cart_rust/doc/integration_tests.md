# MCP Integration Tests

## Overview

This document describes the comprehensive integration test suite for the MCP (Model Context Protocol) server implementation.

## Test Coverage

The integration tests verify the complete MCP protocol implementation with **16 test cases** covering:

### 1. Protocol Handshake & Discovery (4 tests)

- **`test_mcp_sse_endpoint`**: Verifies SSE (Server-Sent Events) endpoint for GET requests
  - Checks `text/event-stream` content type
  - Validates endpoint data format

- **`test_mcp_initialize`**: Tests MCP initialization handshake
  - Protocol version verification
  - Server capabilities announcement
  - Server info metadata

- **`test_mcp_tools_list`**: Validates tool discovery
  - Lists available tools (add_to_cart, checkout)
  - Verifies tool schemas and metadata
  - Checks input validation schemas

- **`test_mcp_resources_list`**: Tests resource discovery
  - Lists available resources (widget HTML)
  - Validates resource URIs and MIME types

### 2. Resource Management (1 test)

- **`test_mcp_resources_read`**: Tests resource content retrieval
  - Reads widget HTML content
  - Validates content structure and metadata

### 3. Tool Execution - Happy Path (3 tests)

- **`test_mcp_tool_call_add_to_cart`**: Basic add to cart functionality
  - Adds multiple items to cart
  - Verifies structured response
  - Checks cart ID generation

- **`test_mcp_tool_call_add_to_cart_aggregation`**: Quantity aggregation
  - Adds same item multiple times
  - Verifies quantities are summed correctly
  - Tests cart persistence across calls

- **`test_mcp_tool_call_checkout`**: Checkout functionality
  - Adds items then checks out
  - Verifies cart is cleared
  - Validates checkout confirmation

### 4. Edge Cases & Error Handling (5 tests)

- **`test_mcp_tool_call_checkout_empty_cart`**: Checkout with no items
  - Handles empty cart gracefully
  - Returns appropriate message

- **`test_mcp_unknown_method`**: Unknown JSON-RPC method
  - Returns proper error code (-32601)
  - Provides "Method not found" message

- **`test_mcp_invalid_json`**: Malformed JSON request
  - Returns parse error (-32700)
  - Handles bad request status

- **`test_mcp_tool_call_unknown_tool`**: Unknown tool name
  - Returns invalid params error (-32602)
  - Provides descriptive error message

- **`test_mcp_tool_call_invalid_arguments`**: Invalid tool arguments
  - Validates input schema
  - Returns appropriate error

### 5. Protocol Features (2 tests)

- **`test_mcp_ping`**: Health check endpoint
  - Verifies ping/pong functionality
  - Tests server responsiveness

- **`test_mcp_notifications_initialized`**: Notification handling
  - Tests notification acknowledgment
  - Validates notification flow

### 6. Multi-Cart Scenarios (1 test)

- **`test_multiple_carts_isolation`**: Cart isolation
  - Creates multiple independent carts
  - Verifies carts don't interfere with each other
  - Tests concurrent cart operations

## Test Architecture

### Test Helper Functions

```rust
fn create_test_app() -> axum::Router
```
Creates a fresh application instance with clean state for each test.

```rust
async fn send_jsonrpc_request(
    app: &mut axum::Router,
    method: &str,
    params: Option<Value>,
    id: i32,
) -> (StatusCode, Value)
```
Helper to send JSON-RPC requests and parse responses.

### Test Isolation

Each test:
- Creates a fresh application instance
- Has independent state
- Can run in parallel
- Doesn't affect other tests

## Running the Tests

### Run all integration tests:
```bash
cargo test --test mcp_integration
```

### Run a specific test:
```bash
cargo test --test mcp_integration test_mcp_initialize
```

### Run all tests (unit + integration):
```bash
cargo test
```

## Test Results

```
running 16 tests
test test_mcp_sse_endpoint ... ok
test test_mcp_initialize ... ok
test test_mcp_tools_list ... ok
test test_mcp_resources_list ... ok
test test_mcp_resources_read ... ok
test test_mcp_tool_call_add_to_cart ... ok
test test_mcp_tool_call_add_to_cart_aggregation ... ok
test test_mcp_tool_call_checkout ... ok
test test_mcp_tool_call_checkout_empty_cart ... ok
test test_mcp_unknown_method ... ok
test test_mcp_invalid_json ... ok
test test_mcp_tool_call_unknown_tool ... ok
test test_mcp_tool_call_invalid_arguments ... ok
test test_mcp_ping ... ok
test test_mcp_notifications_initialized ... ok
test test_multiple_carts_isolation ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured
```

## Coverage Summary

| Category | Tests | Coverage |
|----------|-------|----------|
| Protocol Handshake | 4 | ✅ Complete |
| Resource Management | 1 | ✅ Complete |
| Tool Execution | 3 | ✅ Complete |
| Error Handling | 5 | ✅ Complete |
| Protocol Features | 2 | ✅ Complete |
| Multi-Cart Scenarios | 1 | ✅ Complete |
| **Total** | **16** | **100%** |

## JSON-RPC Error Codes Tested

| Code | Meaning | Test |
|------|---------|------|
| -32700 | Parse error | `test_mcp_invalid_json` |
| -32601 | Method not found | `test_mcp_unknown_method` |
| -32602 | Invalid params | `test_mcp_tool_call_unknown_tool`, `test_mcp_tool_call_invalid_arguments` |

## MCP Methods Tested

| Method | Test(s) |
|--------|---------|
| `initialize` | `test_mcp_initialize` |
| `tools/list` | `test_mcp_tools_list` |
| `resources/list` | `test_mcp_resources_list` |
| `resources/read` | `test_mcp_resources_read` |
| `tools/call` | 6 tests (add_to_cart, checkout variants) |
| `ping` | `test_mcp_ping` |
| `notifications/initialized` | `test_mcp_notifications_initialized` |

## Future Test Enhancements

Potential areas for additional testing:

1. **Performance Tests**: Load testing with many concurrent requests
2. **Stress Tests**: Large cart sizes, many items
3. **Security Tests**: Input sanitization, injection attempts
4. **Concurrency Tests**: Race conditions, parallel cart modifications
5. **Widget Tests**: HTML content validation, template rendering
6. **Persistence Tests**: State recovery, data consistency

## Dependencies

The integration tests require:
- `tower` with `util` feature (for `ServiceExt::oneshot`)
- `axum` for HTTP testing
- `serde_json` for JSON manipulation
- `tokio` for async runtime

---

**Date**: 2025-12-27
**Status**: ✅ All tests passing (16/16)
