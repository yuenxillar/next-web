# SockJS 快速开始指南

## 简介

本指南将帮助您在 5 分钟内开始使用 `next-web-websocket` 的 SockJS 功能。

---

## 为什么需要 SockJS？

在某些环境下，原生 WebSocket 会失效：

- 浏览器不支持或无法连接 WebSocket 时，可以尝试已实现的 XHR Streaming 和 XHR Polling
- ❌ 企业代理或防火墙拦截 `Upgrade` 头
- ❌ 某些代理会关闭看似空闲的长连接

✅ **SockJS 的解决方案**: 优先尝试 WebSocket，不行就自动降级到 HTTP 方案。

### 降级机制

```
WebSocket (最优)
    ↓ (不可用时)
HTTP Streaming (降级 1)
    ↓ (不可用时)
HTTP Long Polling (降级 2)
```

当前服务端仅实现 WebSocket、XHR Streaming 和 XHR Polling；IE 8/9 所需的
HTMLFile、JSONP 和 XDomainRequest 等传输尚未实现，因此不保证支持这些浏览器。

---

## 快速开始

### 1. 添加依赖

在您的 `Cargo.toml` 中添加:

```toml
[dependencies]
next-web-websocket = "0.1.2"
axum = "0.8"
tokio = { version = "1", features = ["full"] }
```

### 2. 创建 SockJS 服务

下面的 Axum 示例只挂载传输端点。实际应用处理器请通过
`get_sockjs_service_with_application_handler` 绑定；如果使用 NextWeb，推荐在
`WebSocketConfigurer` 里启用 `with_sock_js()`：

```rust
registry
    .add_handler(Arc::new(MyWebSocketHandler), vec!["/events".to_string()])
    .with_sock_js()
    .set_websocket_enabled(true);
```

完整的 NextWeb 配置器示例见 [`examples/ws.rs`](../examples/ws.rs)。传输选择由
浏览器里的 SockJS 客户端完成；设置 `set_websocket_enabled(false)` 可以强制测试
HTTP 回退。

```rust
use next_web_websocket::{
    config::sock_js_service_registration::SockJsServiceRegistration,
    sockjs::{router::SockJsRouter, service::SockJsService},
};
use axum::Router;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // 1. 创建 SockJS 配置
    let mut registration = SockJsServiceRegistration::default();
    registration
        .set_heartbeat_time(25000)      // 25秒心跳
        .set_disconnect_delay(5000)     // 5秒超时
        .set_websocket_enabled(true);   // 启用 WebSocket
    
    // 2. 获取 SockJS 服务
    let service: Arc<dyn SockJsService> = Arc::from(registration.get_sockjs_service());
    
    // 3. 创建路由
    let app = Router::new()
        .sockjs("/sockjs", service);  // 自动注册所有端点
    
    // 4. 启动服务器
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();
    
    println!("SockJS server running on http://127.0.0.1:8080");
    axum::serve(listener, app).await.unwrap();
}
```

### 3. 客户端连接

在您的前端代码中:

```javascript
// 引入 SockJS 客户端库
<script src="https://cdn.jsdelivr.net/npm/sockjs-client@1/dist/sockjs.min.js"></script>

<script>
// 连接到 SockJS 服务器
const sock = new SockJS('http://localhost:8080/sockjs');

sock.onopen = function() {
    console.log('✅ Connected to SockJS');
    sock.send('Hello Server!');
};

sock.onmessage = function(e) {
    console.log('📩 Received:', e.data);
};

sock.onclose = function() {
    console.log('❌ Connection closed');
};
</script>
```

### 4. 运行测试

```bash
# 启动 Axum 示例服务器
cargo run -p next-web-websocket --example sockjs_example

# 在包含 sockjs-client 的应用页面中连接
http://localhost:8080/events

# 客户端会自动:
# 1. GET /sockjs/info          - 获取服务器信息
# 2. WS  /sockjs/xxx/yyy/websocket - 尝试 WebSocket
# 3. 如果失败，降级到 HTTP 传输
```

---

## 完整示例

### 服务器端 (Rust)

```rust
use axum::Router;
use next_web_websocket::{
    config::sock_js_service_registration::SockJsServiceRegistration,
    sockjs::{router::SockJsRouter, service::SockJsService},
};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // 配置 SockJS
    let mut registration = SockJsServiceRegistration::default();
    
    // 自定义配置
    registration
        .set_heartbeat_time(25000)              // 心跳间隔 25 秒
        .set_disconnect_delay(5000)             // 5 秒无连接视为断开
        .set_http_message_cache_size(100)       // 缓存 100 条消息
        .set_stream_bytes_limit(128 * 1024)     // 流式传输 128KB 限制
        .set_session_cookie_needed(true)        // 负载均衡依赖会话 Cookie 时启用
        .set_websocket_enabled(true)            // 启用 WebSocket
        .set_suppress_cors(false);              // 启用 CORS
    
    // 设置允许的源（生产环境）
    registration.set_allowed_origins(vec![
        "http://localhost:3000".to_string(),
        "https://yourdomain.com".to_string(),
    ]);
    
    // 获取服务
    let service: Arc<dyn SockJsService> = Arc::from(registration.get_sockjs_service());
    
    // 创建路由
    let app = Router::new()
        .sockjs("/sockjs", service);
    
    // 启动服务器
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .unwrap();
    
    println!("🚀 SockJS server listening on http://0.0.0.0:8080");
    println!("📍 Endpoints:");
    println!("   - GET  http://localhost:8080/sockjs/info");
    println!("   - WS   http://localhost:8080/sockjs/{{server}}/{{session}}/websocket");
    println!("   - POST http://localhost:8080/sockjs/{{server}}/{{session}}/xhr");
    println!("   - POST http://localhost:8080/sockjs/{{server}}/{{session}}/xhr_streaming");
    println!("   - POST http://localhost:8080/sockjs/{{server}}/{{session}}/xhr_send");
    
    axum::serve(listener, app).await.unwrap();
}
```

### 客户端 (HTML + JavaScript)

```html
<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <title>SockJS Demo</title>
    <script src="https://cdn.jsdelivr.net/npm/sockjs-client@1/dist/sockjs.min.js"></script>
    <style>
        body {
            font-family: system-ui, -apple-system, sans-serif;
            max-width: 800px;
            margin: 50px auto;
            padding: 20px;
            background: #0F172A;
            color: #F8FAFC;
        }
        #messages {
            background: #1E293B;
            border: 1px solid #334155;
            border-radius: 8px;
            padding: 20px;
            height: 300px;
            overflow-y: auto;
            margin: 20px 0;
        }
        .message {
            margin: 10px 0;
            padding: 8px 12px;
            background: #0B0E14;
            border-left: 3px solid #38BDF8;
            border-radius: 4px;
        }
        input {
            width: calc(100% - 120px);
            padding: 10px;
            border: 1px solid #334155;
            border-radius: 4px;
            background: #1E293B;
            color: #F8FAFC;
        }
        button {
            padding: 10px 20px;
            border: none;
            border-radius: 4px;
            background: #38BDF8;
            color: #0F172A;
            cursor: pointer;
            font-weight: 600;
        }
        button:hover {
            background: #22D3EE;
        }
        .status {
            display: inline-block;
            width: 10px;
            height: 10px;
            border-radius: 50%;
            margin-right: 8px;
        }
        .status.connected { background: #4ADE80; }
        .status.disconnected { background: #F87171; }
    </style>
</head>
<body>
    <h1>SockJS Demo</h1>
    <div>
        <span class="status disconnected" id="status"></span>
        <span id="status-text">Disconnected</span>
    </div>
    
    <div id="messages"></div>
    
    <div>
        <input type="text" id="messageInput" placeholder="输入消息..." />
        <button onclick="sendMessage()">发送</button>
    </div>
    
    <script>
        const messagesDiv = document.getElementById('messages');
        const statusDot = document.getElementById('status');
        const statusText = document.getElementById('status-text');
        const messageInput = document.getElementById('messageInput');
        
        // 连接到 SockJS
        const sock = new SockJS('http://localhost:8080/sockjs');
        
        sock.onopen = function() {
            console.log('✅ Connected to SockJS');
            addMessage('系统', '已连接到服务器', 'system');
            statusDot.className = 'status connected';
            statusText.textContent = 'Connected';
        };
        
        sock.onmessage = function(e) {
            console.log('📩 Received:', e.data);
            addMessage('服务器', e.data, 'server');
        };
        
        sock.onclose = function(e) {
            console.log('❌ Connection closed:', e);
            addMessage('系统', '连接已断开', 'system');
            statusDot.className = 'status disconnected';
            statusText.textContent = 'Disconnected';
        };
        
        sock.onerror = function(e) {
            console.error('❌ Error:', e);
            addMessage('系统', '连接错误', 'error');
        };
        
        function sendMessage() {
            const message = messageInput.value.trim();
            if (message && sock.readyState === SockJS.OPEN) {
                sock.send(message);
                addMessage('我', message, 'client');
                messageInput.value = '';
            }
        }
        
        function addMessage(from, text, type) {
            const messageEl = document.createElement('div');
            messageEl.className = 'message';
            messageEl.innerHTML = `<strong>${from}:</strong> ${text}`;
            messagesDiv.appendChild(messageEl);
            messagesDiv.scrollTop = messagesDiv.scrollHeight;
        }
        
        // 回车发送
        messageInput.addEventListener('keypress', function(e) {
            if (e.key === 'Enter') {
                sendMessage();
            }
        });
    </script>
</body>
</html>
```

---

## 高级配置

### 1. 自定义传输处理器

```rust
use next_web_websocket::sockjs::transport::{
    TransportHandler,
    WebSocketTransportHandler,
    XhrStreamingTransportHandler,
};

let mut registration = SockJsServiceRegistration::default();

// 只使用 WebSocket 和 XHR Streaming
let handlers: Vec<Arc<dyn TransportHandler>> = vec![
    Arc::new(WebSocketTransportHandler::new()),
    Arc::new(XhrStreamingTransportHandler::new(service.clone())),
];

registration.set_transport_handlers(handlers);
```

### 2. 添加握手拦截器

```rust
use next_web_websocket::server::support::handshake_interceptor::{
    HandshakeInterceptor,
    OriginHandshakeInterceptor,
};

let mut registration = SockJsServiceRegistration::default();

// 添加源站验证拦截器
let interceptor = OriginHandshakeInterceptor::new();
registration.set_interceptors(vec![Arc::new(interceptor)]);
```

### 3. 自定义消息编解码器

```rust
use next_web_websocket::sockjs::codec::{
    SockJsMessageCodec,
    JsonSockJsMessageCodec,
};

let mut registration = SockJsServiceRegistration::default();

// 使用 JSON 编解码器（默认）
let codec = Arc::new(JsonSockJsMessageCodec);
registration.set_message_codec(codec);
```

### 4. CORS 配置

```rust
let mut registration = SockJsServiceRegistration::default();

// 方式 1: 允许特定源
registration.set_allowed_origins(vec![
    "http://localhost:3000".to_string(),
    "https://yourdomain.com".to_string(),
]);

// 方式 2: 使用模式匹配
registration.set_allowed_origin_patterns(vec![
    "https://*.yourdomain.com".to_string(),
]);

// 方式 3: 禁用 CORS（不推荐）
registration.set_suppress_cors(true);
```

---

## 工作原理

### 连接流程

```
客户端                         服务器
  |                              |
  |  1. GET /sockjs/info         |
  |----------------------------->|
  |  {"websocket": true, ...}    |
  |<-----------------------------|
  |                              |
  |  2. WS /sockjs/.../websocket |
  |----------------------------->|
  |  101 Switching Protocols     |
  |<-----------------------------|
  |                              |
  |  3. 'o' (open frame)         |
  |<-----------------------------|
  |                              |
  |  4. a["Hello"]               |
  |<-----------------------------|
  |                              |
  |  5. a["Reply"]               |
  |----------------------------->|
  |                              |
  |  6. 'h' (heartbeat, 每25秒) |
  |<-----------------------------|
  |                              |
  |  7. c[3000,"Bye"]            |
  |<-----------------------------|
  |                              |
```

### SockJS 帧格式

| 帧类型 | 格式 | 说明 | 示例 |
|-------|------|------|------|
| Open | `o` | 连接建立 | `o` |
| Heartbeat | `h` | 保持活跃 | `h` |
| Message | `a[...]` | 消息数组 | `a["Hello","World"]` |
| Close | `c[code,"reason"]` | 关闭连接 | `c[3000,"Go away!"]` |

---

## 常见问题

### Q1: 如何知道当前使用的是哪种传输方式？

**A**: 在浏览器控制台查看:

```javascript
console.log('Transport:', sock._transport.transportName);
// 输出: "websocket", "xhr-streaming", 或 "xhr-polling"
```

### Q2: 如何处理连接断开重连？

**A**: 添加自动重连逻辑:

```javascript
function connect() {
    const sock = new SockJS('http://localhost:8080/sockjs');
    
    sock.onclose = function() {
        console.log('Reconnecting in 3 seconds...');
        setTimeout(connect, 3000);
    };
    
    return sock;
}

const sock = connect();
```

### Q3: 如何在生产环境中使用？

**A**: 关键配置:

```rust
registration
    .set_allowed_origins(vec!["https://yourdomain.com".to_string()])
    .set_suppress_cors(false)
    .set_heartbeat_time(25000)
    .set_disconnect_delay(5000);
```

### Q4: 性能如何？

**A**: 
- 当前没有可引用的并发、内存或延迟基准；实际容量需要按部署配置和消息负载压测。

### Q5: 与原生 WebSocket 的区别？

**A**:

| 特性 | 原生 WebSocket | SockJS |
|-----|---------------|--------|
| 浏览器兼容性 | 支持 WebSocket 的浏览器 | 支持所选 XHR 传输的浏览器 |
| 穿透代理 | 可能失败 | 自动降级 |
| 协议 | `ws://` | HTTP/WS 混合 |
| 开销 | 最小 | 稍高（帧封装） |

---

## 调试技巧

### 1. 启用详细日志

```bash
RUST_LOG=debug cargo run
```

### 2. 查看 /info 端点

```bash
curl http://localhost:8080/sockjs/info
```

输出:
```json
{
  "websocket": true,
  "cookie_needed": true,
  "origins": ["*:*"],
  "entropy": 1234567890
}
```

### 3. 浏览器开发者工具

- Network 标签: 查看 WebSocket 连接和 HTTP 请求
- Console 标签: 查看连接状态和消息

---

## 下一步

- 📖 阅读 [完整 API 文档](./SOCKJS_PHASE2_SUMMARY.md)
- 🔍 查看 [功能检查清单](./FEATURE_CHECKLIST.md)
- 💡 运行 [完整示例](../examples/sockjs_example.rs)
- 🧪 编写自己的测试

---

## 参考资源

- [SockJS 协议规范](https://sockjs.github.io/sockjs-protocol/sockjs-protocol-0.3.3.html)
- [SockJS 客户端文档](https://github.com/sockjs/sockjs-client)
- [Spring Framework WebSocket 文档](https://docs.spring.io/spring-framework/docs/current/reference/html/web.html#websocket-fallback)
- [Axum 框架文档](https://docs.rs/axum/)

---

**祝您使用愉快！** 🎉

如有问题，请查看项目文档或提交 Issue。
