# SockJS Implementation

SockJS core transports for `next-web`: native WebSocket, XHR streaming, and XHR polling. A SockJS JavaScript client chooses an HTTP fallback when its WebSocket attempt is unavailable.

## Overview

SockJS is a browser JavaScript library that provides a WebSocket-like object. It gives you a coherent, cross-browser API for establishing real-time communication between the browser and server, automatically falling back to HTTP-based transports when WebSocket is unavailable.

### Why SockJS?

WebSocket connections can fail in various environments:

- **Browsers without WebSocket**: The client can use the XHR fallback transports implemented here
- **Corporate networks**: Proxies may block WebSocket Upgrade headers
- **Firewalls**: May close connections that appear idle
- **Load balancers**: Some don't properly handle WebSocket

The SockJS client handles transport selection:
1. Attempting native WebSocket first
2. Falling back to HTTP streaming (long-lived connection)
3. Finally degrading to HTTP long polling

This server implements those three transports. It does not currently implement
EventSource, HTMLFile, JSONP, or the legacy XDomainRequest transports, so IE 8/9
compatibility is not guaranteed.

## Architecture

### Components

```
sockjs/
├── codec/              # Message encoding/decoding
│   ├── mod.rs         # SockJsMessageCodec trait
│   └── jackson2.rs    # JSON codec implementation
├── frame.rs           # SockJS frame protocol (o/h/a[]/c[])
├── heartbeat.rs       # Heartbeat mechanism
├── info.rs            # /info endpoint
├── router.rs          # Axum integration
├── service/           # SockJS service implementations
│   ├── mod.rs
│   ├── default_sockjs_service.rs
│   └── transport_handling_sockjs_service.rs
├── session/           # Session management
│   ├── mod.rs
│   ├── sockjs_session.rs
│   └── session_state.rs
└── transport/         # Transport handlers
    ├── mod.rs
    ├── websocket_transport_handler.rs
    ├── xhr_streaming_transport_handler.rs
    └── xhr_polling_transport_handler.rs
```

### Frame Protocol

SockJS uses a simple text-based frame protocol:

- **`o`** - Open frame: Sent when connection is established
- **`h`** - Heartbeat frame: Sent every 25 seconds (default) to keep connection alive
- **`a["msg1","msg2"]`** - Message array: JSON-encoded array of messages
- **`c[3000,"Go away!"]`** - Close frame: Session termination with code and reason

### Transport Handlers

#### 1. WebSocket Transport
- Native WebSocket connection
- Lowest latency, preferred when available
- Bidirectional, full-duplex communication

#### 2. XHR Streaming Transport
- HTTP long-lived connection for server-to-client messages
- Separate HTTP POST requests for client-to-server messages
- Recycles connection after `stream_bytes_limit` bytes

#### 3. XHR Polling Transport
- Traditional HTTP long polling
- Server holds request until message is available
- Client immediately opens new request after receiving response

## Usage

### Basic Configuration

```rust
use next_web_websocket::config::SockJsServiceRegistration;

let mut registration = SockJsServiceRegistration::default();

registration
    .set_heartbeat_time(25_000)           // 25 seconds
    .set_disconnect_delay(5_000)          // 5 seconds
    .set_stream_bytes_limit(128 * 1024)   // 128 KB
    .set_session_cookie_needed(true)      // When load balancing depends on a session cookie
    .set_websocket_enabled(true)          // Allow WebSocket
    .set_http_message_cache_size(100)     // Cache 100 messages
    .set_allowed_origins(vec![
        "https://example.com".to_string(),
        "http://localhost:3000".to_string(),
    ]);
```

### Production Configuration

```rust
let mut registration = SockJsServiceRegistration::default();

registration
    .set_heartbeat_time(30_000)           // 30s for production
    .set_disconnect_delay(10_000)         // 10s disconnect delay
    .set_stream_bytes_limit(256 * 1024)   // 256 KB
    .set_http_message_cache_size(200)     // Larger cache
    .set_allowed_origin_patterns(vec![
        "https://*.example.com".to_string(),
    ]);
```

### HTTP-Only Mode (Testing)

```rust
let mut registration = SockJsServiceRegistration::default();

registration
    .set_websocket_enabled(false)         // Disable WebSocket
    .set_heartbeat_time(15_000)           // More frequent heartbeats
    .set_stream_bytes_limit(64 * 1024);   // Smaller chunks
```

## Configuration Options

| Option | Default | Description |
|--------|---------|-------------|
| `heartbeat_time` | 25,000 ms | Interval between heartbeat frames |
| `disconnect_delay` | 5,000 ms | Time before considering client disconnected |
| `stream_bytes_limit` | 131,072 bytes | Max bytes before recycling HTTP stream |
| `session_cookie_needed` | true | Whether JSESSIONID cookie is required |
| `websocket_enabled` | true | Allow WebSocket transport |
| `http_message_cache_size` | 100 | Max cached messages during polling |
| `client_library_url` | CDN | SockJS client library URL for iframes |
| `suppress_cors` | false | Disable automatic CORS headers |

## Session Management

### Session Lifecycle

```
Client Request → Session Created → Open Frame Sent → Active
                                      ↓
                              Heartbeats Every 25s
                                      ↓
                     No Activity → Disconnect Delay → Closed
```

### Session Registry

Sessions are stored in a concurrent `DashMap<String, Arc<SockJsSession>>`:

- **Session ID**: Supplied by the SockJS client and scoped to this service
- **Thread-safe**: Concurrent registry with per-session async state locks
- **Automatic cleanup**: Background task removes expired sessions

### Heartbeat Mechanism

XHR polling and streaming emit heartbeat frames at the configured interval.
Native WebSocket sessions run a heartbeat task that stops when the transport
closes.

## Client-Server Flow

### Initial Connection

```
1. Client:  GET /sockjs/info
2. Server:  {"websocket": true, "cookie_needed": true, ...}
3. Client:  Chooses transport based on capabilities
4. Client:  Opens WebSocket with GET, or opens an XHR receiver with POST
5. Server:  Sends 'o' (open frame)
6. Client:  Connection established
```

### Message Exchange

```
Client → Server:  ["message1", "message2"]
Server → Client:  a["response1", "response2"]
```

### Heartbeat

```
(25 seconds of silence)
Server → Client:  h
```

### Connection Close

```
Client:   (disconnect or timeout)
Server:   c[3000,"Go away!"]
```

## Endpoints

### /info Endpoint

Returns server capabilities for client negotiation:

```json
{
  "websocket": true,
  "origins": ["*:*"],
  "cookie_needed": true,
  "entropy": 3849572948
}
```

### Transport Endpoints

- `GET /sockjs/{server}/{session}/websocket` - WebSocket upgrade
- `POST /sockjs/{server}/{session}/xhr` - XHR polling receiver
- `POST /sockjs/{server}/{session}/xhr_streaming` - XHR streaming receiver
- `POST /sockjs/{server}/{session}/xhr_send` - Send messages for either XHR transport

GET requests to `xhr` and `xhr_streaming` are also accepted as receive aliases
for compatibility. The official SockJS JavaScript client uses POST for XHR
receivers and sends application messages to `xhr_send`.

## Performance Considerations

### Memory Usage

Session memory depends on queued messages and the configured cache limit; no
load or heap benchmark has been run for this implementation.

### Concurrent Sessions

The implementation uses DashMap, flume channels, and Tokio for concurrent
session and message handling. Capacity has not been load-tested.

### Heartbeat Overhead

- Heartbeats are managed by active transports; cleanup periodically scans sessions.
- Runtime cost depends on active connection count and message volume.

## Integration with Axum

```rust
use axum::Router;
use std::sync::Arc;
use next_web_websocket::sockjs::{
    router::SockJsRouter,
    service::DefaultSockJsService,
    transport::SockJsServiceConfig,
};

let config = SockJsServiceConfig::default();
let service = Arc::new(DefaultSockJsService::new(config));

let app = Router::new()
    .sockjs("/sockjs", service);
```

## NextWeb Registration with `with_sock_js`

Register the same application handler for WebSocket and HTTP fallback routes:

```rust
registry
    .add_handler(Arc::new(EchoHandler), vec!["/events".to_string()])
    .with_sock_js()
    .set_websocket_enabled(true)
    .set_heartbeat_time(25_000);
```

The SockJS client owns transport fallback. Set `set_websocket_enabled(false)`
when testing the HTTP-only path.

## Testing

Run the example:

```bash
cargo run --example sockjs_example
```

Build the library:

```bash
cargo build -p next-web-websocket
```

Run tests:

```bash
cargo test -p next-web-websocket
```

## Compatibility

### Spring Framework

This implementation mirrors Spring's SockJS support:
- Compatible configuration API
- Spring-style `with_sock_js()` registration
- Three built-in transports: WebSocket, XHR streaming, and XHR polling

### SockJS Protocol

Implements the SockJS frames and endpoints used by the three transports listed
above. Rust tests and the official `sockjs-client` 1.6.1 Node transport cover
HTTP polling, streaming, native WebSocket, and automatic fallback. Browser
automation has not been run. EventSource, HTMLFile, JSONP, and XDomainRequest
are not implemented.

## References

- [SockJS Protocol](https://sockjs.github.io/sockjs-protocol/)
- [Spring Framework SockJS](https://docs.spring.io/spring-framework/docs/current/reference/html/web.html#websocket-fallback)
- [sockjs-client](https://github.com/sockjs/sockjs-client)

## License

Copyright 2002-present the original author or authors.

Licensed under the Apache License, Version 2.0.
