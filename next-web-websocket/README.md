# next-web-websocket

WebSocket support for [Next Web](https://crates.io/crates/next-web) applications:
declare handlers the way you declare HTTP routes, have the framework mount them,
and fall back to SockJS when a browser or a proxy will not keep a WebSocket open.

```toml
[dependencies]
next-web = { version = "0.x", features = ["enable-websocket"] }
```

That re-exports this crate as `next_web::ws`. It can also be used on its own:

```toml
[dependencies]
next-web-websocket = "0.2"
```

Which one you want depends on how much of the framework you are using. The
quick start below is a full Next Web application, so it needs `next-web` for
`#[singleton]` and `NextWebApplication`; it imports this crate as
`next_web_websocket::` and is equally correct with `next_web::ws::`. The section
on [SockJS without the framework](#sockjs-without-the-framework) needs nothing
but this crate and `axum`.

## What you get

| | |
|---|---|
| **Lifecycle handlers** | `WebSocketHandler`: `on_open` · `on_message` · `on_error` · `on_close` |
| **Pattern routing** | one handler per path, patterns such as `/ws/chat/{room}` supported |
| **Handshake interceptors** | read the upgrade request, add headers, or refuse the connection |
| **Origin checks** | exact allow-lists or wildcard patterns, enforced by the framework |
| **A per-connection session** | send, close, id, remote address, request headers, request path |
| **SockJS fallback** | `websocket`, `xhr_streaming` and `xhr` transports, heartbeat, CORS preflight, `/info` |
| **Configuration from YAML** | `next.ws.*` |
| **Auto-configuration** | routes are mounted with the application; nothing is wired by hand |

## Quick start

A whole application: one handler, one handshake interceptor, one configurer.

```rust
use std::{collections::HashMap, error::Error, sync::Arc};


use next_web::core::any::AnyValue;
use next_web::core::error::BoxError;
use next_web::core::http::HttpRequest;
use next_web::core::traits::http::http_response::HttpResponse;
use next_web::core::{ApplicationContext, async_trait};
use next_web::macros::bind::singleton;
use next_web::{Application, NextWebApplication};

use next_web::ws::{CloseFrame, Message};
use next_web::ws::config::ws_configurer::WebSocketConfigurer;
use next_web::ws::config::ws_handler_registry::WebSocketHandlerRegistry;
use next_web::ws::server::handshake_interceptor::HandshakeInterceptor;
use next_web::ws::server::support::ws_session::WebSocketSession;
use next_web::ws::ws_handler::{WSResult, WebSocketHandler};

#[derive(Default)]
pub struct MyApplication;

impl Application for MyApplication {}

#[singleton(binds = [Self::into_websocket_handler])]
#[derive(Clone)]
pub struct EchoHandler;

impl EchoHandler {
    fn into_websocket_handler(self) -> Arc<dyn WebSocketHandler> {
        Arc::new(self)
    }
}

#[async_trait]
impl WebSocketHandler for EchoHandler {
    async fn on_open(&self, session: &WebSocketSession) -> WSResult<()> {
        println!("connected: {} from {}", session.id(), session.remote_address());
        session
            .send_message(Message::Text("welcome".into()))
            .await
            .map_err(|error| format!("the client is already gone: {error:?}"))?;
        Ok(())
    }

    async fn on_message(&self, session: &WebSocketSession, message: Message) -> WSResult<()> {
        // Every frame arrives here, including Ping, Pong and Close, so match on
        // the variant rather than assuming it is text.
        if let Message::Text(text) = message {
            session
                .send_message(Message::Text(format!("echo: {text}").into()))
                .await
                .map_err(|error| format!("could not reply: {error:?}"))?;
        }
        Ok(())
    }

    async fn on_error(
        &self,
        _session: &WebSocketSession,
        error: Box<dyn Error + Send + Sync>,
    ) -> WSResult<()> {
        eprintln!("websocket error: {error}");
        Ok(())
    }

    async fn on_close(
        &self,
        session: &WebSocketSession,
        close: Option<CloseFrame>,
    ) -> WSResult<()> {
        println!("disconnected: {} ({close:?})", session.id());
        Ok(())
    }
}

// -- 2. the handshake interceptor (optional) --------------------------------

#[singleton(binds = [Self::into_interceptor])]
#[derive(Clone)]
pub struct AuthInterceptor;

impl AuthInterceptor {
    fn into_interceptor(self) -> Arc<dyn HandshakeInterceptor> {
        Arc::new(self)
    }
}

#[async_trait]
impl HandshakeInterceptor for AuthInterceptor {
    async fn before_handshake(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        _attributes: &mut HashMap<String, AnyValue>,
    ) -> Result<bool, BoxError> {
        // `Ok(false)` refuses the upgrade and the response below is what the
        // client receives. Returning `true` continues to the handshake.
        match request.header("authorization") {
            Some(_) => Ok(true),
            None => {
                response.insert_header("x-reason", "missing authorization");
                Ok(false)
            }
        }
    }
}

// -- 3. the configurer: which paths, which origins, SockJS or not -----------

#[singleton(binds = [Self::into_websocket_configurer])]
#[derive(Clone)]
pub struct MyWebSocketConfigurer;

impl MyWebSocketConfigurer {
    fn into_websocket_configurer(self) -> Box<dyn WebSocketConfigurer> {
        Box::new(self)
    }
}

impl WebSocketConfigurer for MyWebSocketConfigurer {
    fn register_websocket_handlers(
        &mut self,
        _ctx: &mut dyn ApplicationContext,
        registry: &mut dyn WebSocketHandlerRegistry,
    ) {
        registry
            .add_handler(Arc::new(EchoHandler), vec!["/ws".into()])
            .set_allowed_origin_patterns(vec!["https://*.example.com".into()]);
    }
}

#[tokio::main]
async fn main() {
    NextWebApplication::<MyApplication>::default().run().await;
}
```

Connecting to it needs nothing but a URL — `/ws` speaks plain WebSocket:

```javascript
const socket = new WebSocket("ws://127.0.0.1:11000/ws");
socket.onmessage = (event) => console.log(event.data);
socket.onopen = () => socket.send("hello");
```

Two things make this work without any further wiring:

- the `#[singleton(binds = [...])]` attributes put the configurer in the
  application context, where the auto-configuration collects every
  `WebSocketConfigurer` and asks it to register its handlers;
- the same is true of `HandshakeInterceptor`: **every** interceptor registered as
  a singleton runs for **every** registration.

## The handler lifecycle

One handler instance is shared by all connections — the per-connection state lives
in the `WebSocketSession`, not in the handler — so keep the handler stateless, or
synchronise whatever it shares.

When a client connects, this is what happens, in order:

1. The upgrade request is matched against the registered paths.
2. Every handshake interceptor runs, in registration order. The first one that
   returns `false`, or that fails with an error, stops the connect and its
   response is sent to the client.
3. The server sends a `Ping`; if that fails the connection is dropped.
4. The concrete path is matched against the handler mapping. With no match the
   socket is closed.
5. `on_open` runs.
6. Each frame the client sends is passed to `on_message`, **including `Ping`,
   `Pong` and `Close`**. If `on_message` returns an error, `on_error` runs and the
   connection ends.
7. A `Close` frame is remembered and handed to `on_close`.
8. `on_close` runs once, and the connection is finished.

| Callback | Runs when |
|---|---|
| `on_open(&self, session)` | The connection is established, before any message arrives |
| `on_message(&self, session, message)` | A frame arrives from the client |
| `on_error(&self, session, error)` | `on_open` or `on_message` failed, or the socket reported a receive error |
| `on_close(&self, session, close)` | The connection ends; `close` carries the client's close frame when there was one |

Returning `Err` from `on_open` makes the framework call `on_error` and then
`on_close` for that connection. Returning `Err` from `on_close` is only logged.

## Registering handlers

`WebSocketHandlerRegistry::add_handler` takes a handler and the paths it answers,
and returns a builder for everything that is configured per registration:

```rust
registry
    .add_handler(Arc::new(ChatHandler), vec!["/ws/chat/{room}".into()])
    .add_interceptors(vec![Arc::new(RoomGuard)])
    .set_allowed_origins(vec!["https://app.example.com".into()])
    .set_allowed_origin_patterns(vec!["https://*.example.com".into()]);
```

| Method | Effect |
|---|---|
| `add_handler(handler, paths)` | Adds another handler that shares this registration's configuration |
| `add_interceptors(Vec<Arc<dyn HandshakeInterceptor>>)` | Adds interceptors for this registration only |
| `set_allowed_origins(Vec<String>)` | Allows exactly these `Origin` values |
| `set_allowed_origin_patterns(Vec<String>)` | Allows origins matching these patterns. The wildcard applies to the **host**, so `https://*.example.com` accepts `https://api.example.com` and `https://a.b.example.com`, but not `https://evil.com/api.example.com` |
| `with_sock_js()` | Enables the SockJS fallback and returns its own builder |

Setting origins, by either method, installs an origin check on the handshake.
Both an exact list and a pattern list may be given.

Paths are `matchit` patterns, so `/ws/chat/{room}` matches `/ws/chat/general`.
Two caveats:

- **Two different handlers claiming the same path panic at start-up**, as does
  an invalid pattern. Adding the *same* handler again is not a conflict: it
  extends that handler's set of paths. All of this happens while the application
  is being configured, so a mistake surfaces on start-up rather than on the first
  connection.
- **Captured path parameters are matched but not exposed.** `session.path()`
  gives you the concrete request path, so `/ws/chat/general` is available, but
  the mapping does not hand you `room = "general"`; split the path yourself if you
  need the value.

## Handshake interceptors

An interceptor runs before the upgrade completes — for every registration when it
is registered as a singleton, or for one registration when it is passed to
`add_interceptors`. Its return value and what it does with the other arguments
each have a defined effect:

| What it does | What happens |
|---|---|
| Returns `Ok(true)` | The handshake continues |
| Returns `Ok(false)` | The connection is refused and its `response` is sent |
| Returns `Err(..)` | The connection is refused, and the error is logged with the client address, the path and its position in the chain |
| Writes to `response`, then accepts | Its headers join the `101` response |
| Writes to `response`, then refuses | Its headers and status **are** the refusal; the default is `403 Forbidden` |
| Writes to `attributes` | The values are on the session the handler is given, readable with `attributes()` and `attribute(name)` |

A refusal that sets no status answers `403 Forbidden` rather than `200 OK`, so a
client never sees a success for a connection that was never made. A header the
handshake sets itself cannot be overridden, so an interceptor cannot break the
protocol by naming one of its own headers.

```rust
#[async_trait]
impl HandshakeInterceptor for AuthInterceptor {
    async fn before_handshake(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        attributes: &mut HashMap<String, AnyValue>,
    ) -> Result<bool, BoxError> {
        let Some(token) = request.header("authorization") else {
            response.set_status_code(StatusCode::UNAUTHORIZED);
            return Ok(false); // refused, with a status of our own
        };

        match self.verify(token).await {
            Ok(principal) => {
                // Readable by the handler as `session.attribute("principal")`.
                attributes.insert("principal".to_owned(), AnyValue::from(principal));
                Ok(true)
            }
            // Failing closed is the right default, and this is logged for you.
            Err(error) => Err(error.into()),
        }
    }
}
```

## The session

| Method | Use |
|---|---|
| `send_message(Message) -> Result<(), Message>` | Queues a frame to the client; returns it back if the client is gone |
| `close()` | Sends a normal-closure `Close` frame and stops the send task |
| `is_open() -> bool` | Whether the outgoing channel is still connected |
| `id() -> &Uuid` | The connection's identity, stable for its life |
| `remote_address() -> &SocketAddr` | The client's TCP address |
| `header() -> &HeaderMap` | The upgrade request's headers, e.g. for a cookie or user agent |
| `path() -> &str` | The concrete request path |
| `attributes() -> &HashMap<String, AnyValue>` | Everything the handshake interceptors set |
| `attribute(name) -> Option<&AnyValue>` | One of them, by name |

`send_message` is safe to call from anywhere — another task, a timer, a broadcast
loop — because it writes to a channel that a dedicated task drains to the socket.
`WebSocketSession` is `Clone` and cheap to clone, so a handler can hand a session
to a background task.

## Sending to many clients

The crate has no built-in rooms or broadcast, and deliberately so: it does not
know which connections you consider a group. The pattern that works is to keep
your own registry of sessions, because `WebSocketSession` is `Clone` and can send
from any task:

```rust
use std::sync::Arc;

use dashmap::DashMap;
use next_web::core::Uuid;
use next_web_websocket::server::support::ws_session::WebSocketSession;

#[derive(Clone, Default)]
pub struct Room {
    members: Arc<DashMap<Uuid, WebSocketSession>>,
}

impl Room {
    pub fn join(&self, session: &WebSocketSession) {
        self.members.insert(*session.id(), session.clone());
    }

    pub fn leave(&self, session: &WebSocketSession) {
        self.members.remove(session.id());
    }

    pub async fn broadcast(&self, text: &str) {
        for member in self.members.iter() {
            // A send only fails when that client is already gone; drop it then.
            let _ = member
                .send_message(next_web_websocket::Message::Text(text.into()))
                .await;
        }
    }
}
```

Register the room as a singleton, `join` in `on_open`, `leave` in `on_close` and
call `broadcast` from `on_message`. Remove a member as soon as a send fails, or
the map keeps connections that no longer exist.

This snippet needs `dashmap` in your own `Cargo.toml`; the crate does not
re-export it. It is worth the dependency here precisely because a broadcast
awaits while it walks the members, and a `RwLock<HashMap<_, _>>` would hold its
guard across those awaits.

## SockJS

A WebSocket that a corporate proxy or an old browser cannot carry is what SockJS
is for: the client tries a real WebSocket first, and falls back to HTTP
transports that look like ordinary requests.

```rust
registry
    .add_handler(Arc::new(EchoHandler), vec!["/ws".into()])
    .with_sock_js()
    .set_heartbeat_time(25_000)
    .set_websocket_enabled(true)
    .set_allowed_origin_patterns(vec!["http://localhost:*".into()]);
```

SockJS is served on the registered paths, so the client connects to the same URL:

```html
<script src="https://cdn.jsdelivr.net/npm/sockjs-client@1/dist/sockjs.min.js"></script>
<script>
  const sock = new SockJS("http://127.0.0.1:11000/ws");
  sock.onopen = () => sock.send("hello");
  sock.onmessage = (event) => console.log(event.data);
</script>
```

The client appends the SockJS path itself. These are the endpoints mounted under a
base path:

| Endpoint | Purpose |
|---|---|
| `GET {base}/info` | Server capabilities, including whether WebSocket is enabled |
| `GET {base}/{server}/{session}/websocket` | The native WebSocket transport |
| `GET\|POST {base}/{server}/{session}/{transport}` | The HTTP transports |

Implemented transports:

| Transport | URL segment | Notes |
|---|---|---|
| WebSocket | `websocket` | Preferred; disable with `set_websocket_enabled(false)` |
| XHR streaming | `xhr_streaming` | One long-lived response, recycled at `stream_bytes_limit` |
| XHR polling | `xhr` (receive) and `xhr_send` (send) | Request/response pairs |

Frames are the SockJS ones, and are produced for you — a handler only ever sees
`Message`:

| Frame | Meaning |
|---|---|
| `o` | Open |
| `h` | Heartbeat, every `heartbeat_time` (default 25 s) |
| `a["one","two"]` | Application messages |
| `c[3000,"reason"]` | Close; codes 3000 normal, 3001 going away, 3002 protocol error, 3003 unsupported data |

`xhr_streaming` begins with the 2 KB `h` prelude the protocol requires, then `o`.

Handshake interceptors and origin checks apply to the SockJS HTTP transports
exactly as they do to a WebSocket upgrade, so an authorisation interceptor
protects both without any extra work.

`with_sock_js()` **copies** the registration's interceptors and origin settings
into the SockJS service at the moment it is called, so set those first and
configure SockJS itself afterwards:

```rust
registry
    .add_handler(Arc::new(EchoHandler), vec!["/ws".into()])
    .add_interceptors(vec![Arc::new(AuthInterceptor)])
    .set_allowed_origin_patterns(vec!["https://*.example.com".into()])
    .with_sock_js()                  // the two lines above are copied here
    .set_heartbeat_time(25_000);     // SockJS's own settings go after
```

Anything set on the WebSocket registration after `with_sock_js()` reaches the
plain WebSocket path but not SockJS. Interceptors registered as singletons are
unaffected by this: they are applied to every registration regardless of order.

### SockJS settings

`with_sock_js()` returns a `SockJsServiceRegistration`; every setter returns
`&mut Self` for chaining, and the defaults are the Spring ones.

| Setter | Default | Effect |
|---|---|---|
| `set_websocket_enabled(bool)` | `true` | Whether the WebSocket transport is offered |
| `set_heartbeat_time(u64)` | `25_000` | Heartbeat interval, ms |
| `set_disconnect_delay(u64)` | `5_000` | Grace before a client with no open poll is treated as gone, ms |
| `set_stream_bytes_limit(usize)` | `128 * 1024` | Bytes per streaming response; `1` forces polling |
| `set_http_message_cache_size(usize)` | `100` | How many frames the delivery queue holds; past that the oldest are dropped, for a client that has stopped collecting |
| `set_session_cookie_needed(bool)` | `true` | Reported in `/info` |
| `set_client_library_url(String)` | jsDelivr `sockjs.min.js` | Reserved for iframe transports |
| `set_suppress_cors(bool)` | `false` | Stop adding CORS headers automatically |
| `set_session_pinning(bool)` | `true` | Whether a session may only be used from the address that created it. Does nothing behind a reverse proxy, where every client arrives as the proxy |
| `set_session_identity_attribute(name)` | unset | Attribute that identifies who a session belongs to; only a request presenting the same value may use it. This is the check that survives a reverse proxy |
| `set_allowed_origins(Vec<String>)` | empty | Allowed origins for the SockJS endpoints |
| `set_allowed_origin_patterns(Vec<String>)` | empty | Allowed origin patterns |
| `set_interceptors(Vec<Arc<dyn HandshakeInterceptor>>)` | empty | Interceptors for the SockJS endpoints |
| `set_message_codec(Arc<dyn SockJsMessageCodec>)` | Jackson2 | Encoding of `a[...]` frames |
| `set_transport_handlers(Vec<Arc<dyn TransportHandler>>)` | — | **Replaces** the default transports |
| `set_transport_handler_overrides(Vec<Arc<dyn TransportHandler>>)` | — | Overrides individual defaults |
| `set_task_scheduler(Arc<dyn Send + Sync>)` | — | Kept for configuration compatibility; heartbeats use Tokio |

`set_transport_handlers` and `set_transport_handler_overrides` are mutually
exclusive: giving both panics at configuration time.

### SockJS without the framework

SockJS can be mounted on a bare `axum::Router`, without the dependency injection
the rest of the crate assumes — this is what
[`examples/sockjs_example.rs`](examples/sockjs_example.rs) does:

```rust
use std::sync::Arc;

use next_web_websocket::{
    config::SockJsServiceRegistration,
    sockjs::{router::SockJsRouter, service::SockJsService},
};

let mut registration = SockJsServiceRegistration::default();
registration.set_websocket_enabled(true);

let service: Arc<dyn SockJsService> =
    Arc::from(registration.get_sockjs_service_with_application_handler(Arc::new(EchoHandler)));

let app = axum::Router::new().sockjs("/events", service);
let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
axum::serve(listener, app).await?;
```

Run it and point the SockJS client at `http://127.0.0.1:8080/events`:

```text
cargo run -p next-web-websocket --example sockjs_example
```

## Configuration

Settings live under the `next.ws` prefix, in the same YAML the rest of the
framework reads. Keys are written in `snake_case`, and the `kebab-case` spelling
of a key is accepted too.

```yaml
next:
  ws:
    max_msg_size: 1048576          # bytes, default 64 MiB
    max_write_buffer_size: 1048576 # bytes, default usize::MAX
    msg_channel_capacity: 128      # default 128; 0 means unbounded
    max_session_idle_timeout: 30   # seconds of client silence before the server closes
```

| Key | Type | Default | Effect |
|---|---|---|---|
| `max_msg_size` | `usize` | `64 << 20` (64 MiB) | Largest frame accepted |
| `max_write_buffer_size` | `usize` | `usize::MAX` | Write buffer watermark |
| `msg_channel_capacity` | `u32` | `128` | Outgoing queue depth per connection; `0` is unbounded. Applies to the plain WebSocket path only — the SockJS transports hard-code `128` |
| `max_session_idle_timeout` | `u64` | unset | Seconds a client may stay silent before the server closes the connection; unset or `0` leaves it open. Applies to the plain WebSocket path |

The type is `next_web_websocket::autoconfigure::ws_properties::WebSocketProperties`,
and it can also be built in code with the same setters.

## Limitations

Worth knowing before you design around this crate:

- **No broadcast, no rooms, no presence.** Keep your own registry of sessions;
  see [Sending to many clients](#sending-to-many-clients).
- **A SockJS session has one receiver, and a new one replaces the old.** Every
  receive request takes the session over; the one it replaced ends its response
  immediately and sends nothing, because the client that opened it has already
  moved on to the new one. A request that gave up waiting and re-polled therefore
  gets a clean handover rather than two responses splitting the frames. The same
  applies across transports: a client that switches between them, or reconnects,
  keeps its session id, is served on the new connection, and the socket it
  replaced is closed so its receive loop does not linger. Its id in the URL is
  still chosen by the client, so this is not authentication: a request
  that guesses or learns another client's id reaches that session — but only from
  the IP that first used it, and a later caller cannot rebind it. A request from
  anywhere else is answered `404` (which the SockJS client reads as "start a new
  session"). Turn the IP pin off with `set_session_pinning(false)` if your clients
  legitimately change address mid-session; note that behind a proxy every client
  shares the proxy's address, which makes the pin a no-op.
- **Behind a proxy, bind the session to an identity instead of an address.**
  `set_session_identity_attribute` names an attribute your handshake interceptor
  fills in, and from then on only a request presenting the same value may use that
  session — the session is not handed over, and the request is answered `404`. The
  value reads a `String`, number or boolean; anything else, such as a boxed value,
  cannot be compared and is refused rather than treated as equal, so a
  misconfiguration fails loudly instead of silently. The request that creates a
  session always owns it. This is the check to use when the application cannot see
  client addresses, and it is off until you ask for it.

  ```rust
  // The interceptor puts the authenticated principal in the agreed attribute:
  attributes.insert(
      next_web_websocket::SESSION_IDENTITY_ATTRIBUTE.to_owned(),
      AnyValue::from(principal.user_id()),
  );

  // And the registration turns the check on:
  .with_sock_js()
  .set_session_identity_attribute(next_web_websocket::SESSION_IDENTITY_ATTRIBUTE);
  ```

  Neither this nor the address pin is authentication. Both decide *which* session a
  request may reach; neither decides *who* the caller is. Authenticate in the
  interceptor first.
- **Path parameters are not exposed.** Match the pattern, then parse
  `session.path()` yourself.
- **One handler per path.** A second registration for the same pattern panics at
  start-up.
- **`jsonp` is not implemented.** A request for it returns `404`.
  `TransportType::EventSource` also exists in the enum, and `eventsource` is
  parsed out of the URL, but no transport handler is registered for it — only
  `websocket`, `xhr_streaming` and `xhr` are implemented. The module-level docs in
  `sockjs::transport` claim more than the code provides. A request for a transport
  that has no handler is answered `404` without creating a session.
- **`client_library_url` does nothing yet.** It exists for the iframe transports
  (`iframe.html`, `htmlfile`), which are not implemented, so nothing serves it.
- **`max_session_idle_timeout` applies to the plain WebSocket path only.** There
  it closes a connection whose client has gone quiet, after the configured number
  of seconds. The SockJS sessions keep their own, unrelated `disconnect_delay`,
  and a SockJS WebSocket whose peer is silent is not reclaimed by this setting.
- **`is_open()` documents itself wrongly.** Its doc comment claims it always
  returns `false`; the implementation returns whether the outgoing channel is
  still connected. Trust the code, not that comment.

## Modules

| Module | Contents |
|---|---|
| `config` | `WebSocketConfigurer`, `WebSocketHandlerRegistry`, `WebSocketHandlerRegistration`, `SockJsServiceRegistration`, `DefaultWebSocketHandlerRegistry` |
| `ws_handler` | `WebSocketHandler`, `WSResult` |
| `server` | `HandshakeInterceptor`, `WebSocketSession`, `WebSocketHandlerMapping`, `WebSocketContext`, `OriginHandshakeInterceptor` |
| `sockjs` | `SockJsService`, `SockJsSession`, `SockJsFrame`, transports, `SockJsRouter` |
| `ws_handle` | The upgrade handler and the per-connection loop |
| `autoconfigure` | `WebsocketAutoConfiguration`, `WebSocketProperties` |

`Message`, `WebSocket` and `WebSocketUpgrade` are re-exported from
`axum::extract::ws` at the crate root, so `next_web_websocket::Message` is the
frame type used throughout.

## Examples

```text
# A SockJS server on http://127.0.0.1:8080/events, no framework needed.
cargo run -p next-web-websocket --example sockjs_example
```

The framework-integrated version is in the workspace examples crate:

```text
# Registers /ws with an origin pattern and a handshake interceptor.
cargo run -p next-web-examples --bin ws
```

## License

Same as the rest of the Next Web workspace.
