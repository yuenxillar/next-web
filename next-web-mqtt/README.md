# Next Web MQTT

MQTT support for `next-web`. The crate is used together with `next-web`; the
client is auto-configured from the `next.mqtt` properties, subscribes the
configured topics and dispatches every received message to the topic listeners
of the application.

## Configuration

Add the properties to `resources/application.yaml`:

```yaml
next:
    mqtt:
        # Broker URL. The scheme selects the transport.
        url: tcp://127.0.0.1:1883
        client_id: next-web-mqtt
        username: "user1"
        password: "123"
        keep_alive: 60
        connect_timeout: 10
        clean_session: true
        topics:
            - topic: "test/#"
              qos: 1
            - topic: "testtopic/#"
              qos: 0
```

`url` is the broker URL, and its scheme selects the transport:

| Scheme | Transport | Default port |
| --- | --- | --- |
| `tcp`, `mqtt` | plain TCP | `1883` |
| `ssl`, `mqtts` | TCP over TLS | `8883` |
| `ws` | WebSocket | `80` |
| `wss` | WebSocket over TLS | `443` |

When the URL has no port the default port of its scheme is used, and a
WebSocket URL may carry the handshake path, for example
`wss://broker.example.com:8000/mqtt`. The `ws` and `wss` schemes require the
`ws` feature:

```toml
next-web-mqtt = { version = "0.1", features = ["ws"] }
```

Every property is optional. The broker defaults to `tcp://127.0.0.1:1883`, and
the client identifier is generated when `client_id` is not set:

| Property | Default | Description |
| --- | --- | --- |
| `url` | `tcp://127.0.0.1:1883` | Broker URL. |
| `client_id` | generated | MQTT client identifier. |
| `username`, `password` | none | Credentials. |
| `topics` | none | Topic filters to subscribe, each with an optional `qos`. |
| `keep_alive` | `60` | Keep alive interval in seconds. |
| `connect_timeout` | `10` | Connection timeout in seconds. |
| `clean_session` | `true` | Start a clean session. |
| `cap` | `2048` | Capacity of the bounded client channel. |
| `max_retries` | `5` | Connection retries before the application stops. |

## Usage

The client is registered as the `MqttClient` singleton. It is only created when
the application registers at least one `TopicListener`, so an application that
does not consume MQTT messages does not connect to a broker.

```rust
use std::sync::Arc;

use axum::Json;
use next_web::extract::find_singleton::FindSingleton;
use next_web::macros::bind::{post_mapping, singleton};
use next_web::{Application, NextWebApplication};
use next_web_core::async_trait;
use next_web_mqtt::{MqttClient, TopicListener};

#[derive(Default)]
struct TestApplication;

impl Application for TestApplication {}

#[post_mapping(path = "/publish")]
async fn publish_message(
    FindSingleton(mqtt_client): FindSingleton<MqttClient>,
    Json(msg): Json<String>,
) -> impl IntoResponse {
    mqtt_client.publish("test/1/event", msg).await.ok();
    "Ok"
}

/// A listener is registered as `Arc<dyn TopicListener>`.
#[singleton(binds = [Self::into_topic_listener])]
#[derive(Clone)]
pub(crate) struct TestTopicListener;

impl TestTopicListener {
    fn into_topic_listener(self) -> Arc<dyn TopicListener> {
        Arc::new(self)
    }
}

#[async_trait]
impl TopicListener for TestTopicListener {
    fn topic(&self) -> &'static str {
        "test/+/event"
    }

    async fn on_message(&self, topic: &str, message: &[u8]) {
        println!(
            "Received message, Topic: {}, Data Content: {:?}",
            topic,
            String::from_utf8_lossy(message)
        );
    }
}

#[tokio::main]
async fn main() {
    NextWebApplication::<TestApplication>::default().run().await
}
```

## Topic routing

A `TopicListener` declares the filter it handles:

- an exact topic (`sensors/room1`) is matched by name,
- `+` matches a single level (`sensors/+/temperature`),
- `#` matches the rest of the levels (`sensors/#`), including the parent topic.

A message is dispatched to every listener whose filter matches, so several
listeners may consume the same message. As MQTT requires, a `#` or `+` at the
root does not match the `$`-prefixed broker topics, while an explicit `$SYS/#`
filter does.

## Interceptors

A `MessageInterceptor` runs before the dispatch and can drop a message by
returning `false` from `message_entry`. Register one as
`Box<dyn MessageInterceptor>` to install it.

## Features

- `ws` - enables the `ws` and `wss` broker URLs.
- `trace-log` - logs the topics of received messages that no listener matches.
