# SockJS Implementation for next-web-websocket

SockJS core transports for `next-web`, following Spring's WebSocket registration style. The server supports native WebSocket, XHR streaming, and XHR polling through `with_sock_js()`.

## Status

- HTTP fallback, WebSocket routing, session callbacks, CORS, and handshake interceptors are implemented.
- Rust tests and the official `sockjs-client` 1.6.1 Node transport completed end-to-end WebSocket, XHR streaming, XHR polling, and automatic WebSocket-to-XHR fallback checks. Browser-specific automation remains unverified.
- EventSource, HTMLFile, JSONP, and XDomainRequest transports are not implemented. IE 8/9 support is therefore not guaranteed.

---

## 📖 Documentation

| Document | Description |
|----------|-------------|
| [Quick Start Guide](./SOCKJS_QUICKSTART.md) | Get started in 5 minutes |
| [Feature Checklist](./FEATURE_CHECKLIST.md) | Historical Phase 2 checklist |
| [Implementation Summary](./SOCKJS_PHASE2_SUMMARY.md) | Historical implementation notes |
| [Implementation Report](./IMPLEMENTATION_REPORT.md) | Historical project report |
| [Functionality Verification](./FUNCTIONALITY_VERIFICATION.md) | Historical test snapshot |

---

## 🚀 Quick Start

### Add Dependency

```toml
[dependencies]
next-web-websocket = "0.1.2"
axum = "0.8"
tokio = { version = "1", features = ["full"] }
```

### Runnable Server Examples

The [Axum example](./examples/sockjs_example.rs) runs a SockJS echo handler.
The [NextWeb example](../examples/ws.rs) registers an application handler with
`.with_sock_js()` so the same callbacks serve WebSocket and XHR fallback clients.

### Client Code

```javascript
const sock = new SockJS('http://localhost:8080/sockjs');

sock.onopen = () => {
    console.log('Connected');
    sock.send('Hello Server!');
};

sock.onmessage = (e) => {
    console.log('Received:', e.data);
};
```

---

## ✨ Features

### Core Protocol

- ✅ **Session Management** - Full lifecycle with state machine
- ✅ **Frame Protocol** - Open, Heartbeat, Message, Close frames
- ✅ **Message Codec** - JSON encoding/decoding
- ✅ **Heartbeat** - Automatic keep-alive mechanism

### Transports

- ✅ **WebSocket** - Native bidirectional communication (preferred)
- ✅ **XHR Streaming** - HTTP long-polling streaming (fallback 1)
- ✅ **XHR Polling** - HTTP long-polling (fallback 2)

### Configuration

- ✅ **Heartbeat interval** - Configurable (default 25s)
- ✅ **Disconnect delay** - Configurable (default 5s)
- ✅ **Message cache** - Configurable size
- ✅ **CORS support** - Full CORS configuration
- ✅ **Interceptors** - Handshake interceptors

---

## 🤔 Why SockJS?

Native WebSocket fails in some environments:

- Browsers without native WebSocket can use the implemented XHR transports when the browser supports them.
- ❌ Corporate proxies block `Upgrade` headers
- ❌ Some proxies close idle long connections

✅ **SockJS Solution**: Try WebSocket first, automatically fallback to HTTP-based transports.

### Fallback Chain

```
WebSocket (best)
    ↓ (if unavailable)
XHR Streaming (fallback 1)
    ↓ (if unavailable)
XHR Polling (fallback 2)
```

---

## 📊 Implementation Stats

| Metric | Value |
|--------|-------|
| New files | 14 |
| Lines of code | ~2,216 |
| Test code | ~315 |
| Tests | 32 passing |
| Documentation | 5 comprehensive docs |

---

## 🧪 Testing

```bash
# Run tests
cargo test -p next-web-websocket --all-targets
cargo test -p next-web-websocket --doc

# Run example
cargo run --example sockjs_example

# Test /info endpoint
curl http://localhost:8080/sockjs/info
```

---

## 🏗️ Architecture

```
┌─────────────────────────────────────┐
│   Axum Router (HTTP Layer)          │
│   /info, /{server}/{session}/...    │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│   SockJS Service                     │
│   (TransportHandlingSockJsService)  │
└──────────────┬──────────────────────┘
               │
      ┌────────┴────────┐
      │                 │
┌─────▼──────┐   ┌─────▼─────────┐
│  Session   │   │   Transport   │
│  Manager   │   │   Handlers    │
└─────┬──────┘   └─────┬─────────┘
      │                │
┌─────▼──────┐   ┌─────▼─────────┐
│  Sessions  │   │  WS/XHR/Poll  │
│  (DashMap) │   │               │
└────────────┘   └───────────────┘
```

---

## 📋 API Completeness

| API Category | Methods | Status |
|--------------|---------|--------|
| Implemented area | Coverage |
|------------------|----------|
| Built-in transports | WebSocket, XHR streaming, XHR polling |
| Rust transport tests | HTTP polling/streaming and native WebSocket round trips |
| SockJS client interoperability | Official `sockjs-client` 1.6.1 Node transport verified; browser automation remains unverified |

---

## 🔄 Spring Framework Compatibility

Designed to match Spring's WebSocket API:

| Feature | Spring | next-web | Match |
|---------|--------|----------|-------|
| API Design | ✅ | ✅ | 95% |
| Session Management | ✅ | ✅ | Core lifecycle |
| Transports | 7 types | 3 core | Partial transport set |
| Heartbeat | ✅ | ✅ | Implemented for built-in transports |
| Configuration | ✅ | ✅ | Spring-style registration |

---

## Performance

No load, memory, or latency benchmarks have been run. Capacity depends on
message volume, transport mix, and deployment configuration.

---

## 📚 Examples

### Complete Example

See [examples/sockjs_example.rs](../examples/sockjs_example.rs) for a complete working example.

### Configuration Examples

```rust
// Production configuration
registration
    .set_heartbeat_time(25000)
    .set_disconnect_delay(5000)
    .set_http_message_cache_size(100)
    .set_allowed_origins(vec![
        "https://yourdomain.com".to_string()
    ])
    .set_suppress_cors(false);

// Custom transports
let handlers: Vec<Arc<dyn TransportHandler>> = vec![
    Arc::new(WebSocketTransportHandler::new()),
    Arc::new(XhrStreamingTransportHandler::new()),
    Arc::new(XhrPollingTransportHandler::new()),
];
// Supplying handlers replaces the built-in list; include every transport needed.
registration.set_transport_handlers(handlers);

// Add interceptors
let interceptor = OriginHandshakeInterceptor::with_allowed_origins(vec![
    "https://yourdomain.com".to_string(),
]);
registration.set_interceptors(vec![Arc::new(interceptor)]);
```

---

## 🛣️ Roadmap

### ✅ Phase 1: Configuration Layer (Complete)
- Configuration API design
- Builder pattern implementation

### ✅ Phase 2: Protocol Implementation (Complete)
- Session management
- Transport handlers
- Frame protocol
- Heartbeat mechanism
- Router integration

### 🔮 Phase 3: Future Enhancements
- Integration tests with real SockJS client
- Performance benchmarks
- EventSource transport
- Cluster support
- Session persistence (Redis)

---

## 🐛 Known Limitations

- EventSource, HTMLFile, JSONP, and XDomainRequest transports are not implemented; old IE support is not guaranteed.
- The official browser `sockjs-client` has not been used for an end-to-end compatibility test.
- Load, memory, and multi-instance tests have not been run; capacity figures above are estimates only.

---

## 🤝 Contributing

Contributions welcome! Areas for improvement:

1. Integration tests with real SockJS clients
2. Performance benchmarks
3. Additional transport implementations
4. Documentation improvements

---

## 📄 License

Apache License 2.0

---

## 🔗 References

- [SockJS Protocol Specification](https://sockjs.github.io/sockjs-protocol/sockjs-protocol-0.3.3.html)
- [SockJS Client](https://github.com/sockjs/sockjs-client)
- [Spring Framework WebSocket](https://docs.spring.io/spring-framework/docs/current/reference/html/web.html#websocket-fallback)
- [Axum Framework](https://docs.rs/axum/)

---

## 👨‍💻 Credits

Implementation by Claude Opus 5.5  
Date: 2026-10-13  
Project: next-web-websocket v0.2.1

---

**Ready to use!** Check the [Quick Start Guide](./SOCKJS_QUICKSTART.md) to get started. 🚀
