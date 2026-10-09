English | [简体中文](README.zh-CN.md)

# Next Web

> Build web applications with joy and simplicity.

[![Crates.io](https://img.shields.io/crates/v/next-web.svg)](https://crates.io/crates/next-web)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

Next Web is a Rust web application framework built on Tokio and Axum. It borrows proven application organization ideas from the Spring ecosystem and brings common web-service infrastructure into a unified application model through macros, auto-configuration, dependency injection, configuration properties, lifecycle management, events, and scheduling.

Its goal is not only to answer "how do I write a route?", but also "how do I organize a Rust web application that keeps growing?" You can start with a small HTTP service and later add configuration binding, singleton management, security, internationalization, data access, messaging, API documentation, and observability as needed.

## Core Capabilities

- **Macro-driven development**: routes, dependency injection, configuration properties, scheduling, retries, and validation are declared with procedural macros, keeping application code close to the business domain.
- **Application context and singleton management**: `ApplicationContext` manages application components, while `#[singleton]` and `FindSingleton` make dependencies explicit.
- **Configuration and environment variables**: YAML / properties files, profiles, configuration property binding, `${property:default}`, and `#{ENV_VAR:default}` placeholders.
- **Auto-configuration and conditional wiring**: organize starters and application-level configuration with `AutoConfiguration` and `#[auto_configuration]`.
- **Modular by design**: the core, context, macros, and integrations live in separate crates, so capabilities can be enabled and adopted as needed.
- **Built for real services**: provides or integrates Redis, databases, MQTT, message queues, mail, WebSocket, OpenAPI, security, scheduling, XXL-JOB, and more.
- **Based on a mature ecosystem**: runs on Tokio and Axum, using mimalloc on Windows and jemalloc on Linux by default.

## Quick Start

### Requirements

- Rust 1.98.1 or newer
- Cargo

### Add Dependencies

```toml
[dependencies]
next-web = "0.2"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

To enable most optional capabilities at once, use the `full` feature:

```toml
[dependencies]
next-web = { version = "0.2", features = ["full"] }
```

### Minimal Example

```rust
use next_web::{
    Application, NextWebApplication,
    macros::bind::get_mapping,
};

#[derive(Default)]
struct TestApplication;

impl Application for TestApplication {}

#[get_mapping(path = "/hello")]
async fn hello() -> &'static str {
    "Hello World!"
}

#[tokio::main]
async fn main() {
    NextWebApplication::<TestApplication>::default().run().await
}

```

Run it:

```bash
cargo run
```

The default port is `11000`. Use `next.server.address` and `next.server.port` to change the listening address.

## Configuration and Environment Variables

By default, the framework loads `application.yaml`, `application.yml`, or `application.properties`. The configuration sources can be changed with `next.config.location`, `next.config.additional_location`, and `next.config.import`. For example:

```yaml
next:
  server:
    address: 0.0.0.0
    port: 11000
```

Configuration values support two kinds of placeholders:

- `${property:default}`: resolves a configuration property, for example `${next.application.name:demo}`.
- `#{ENV_VAR:default}`: resolves a process environment variable, for example `#{DATABASE_URL:jdbc:mysql://localhost:3306/default}`. The text after the first colon is the default value. If the variable is missing and no default is declared, configuration binding returns a clear error.

In YAML, a value that starts with `#` must be quoted:

```yaml
next:
  datasource:
    url: "#{DATABASE_URL:jdbc:mysql://localhost:3306/default}"
```

## Modules

The project is organized as a workspace, keeping the framework itself separate from its integrations so they can be combined as needed. The main crates are:

| Crate | Description |
| --- | --- |
| `next-web` | Application entry point, web server, configuration loading, lifecycle, events, auto-configuration, and diagnostics |
| `next-web-core` | Core abstractions: environment and configuration binding, HTTP abstractions, filters, metrics, messaging, and common utilities |
| `next-web-context` | `ApplicationContext`, singletons, `MessageSource`, property sources, and placeholder support |
| `next-web-singletons` | Singleton registration, creation, and dependency resolution |
| `next-web-macros` | Procedural macros for routes, dependency injection, configuration properties, scheduling, retries, and desensitization |
| `next-web-security` | Security, authentication, and authorization extensions |
| `next-web-websocket` | WebSocket server support |
| `next-web-open-api`  | OpenAPI / Swagger documentation based on utoipa |
| `next-web-data-redis` | Redis integration, with optional distributed locking |
| `next-web-data-database` | MySQL / PostgreSQL data access and transaction support |
| `next-web-mqtt` / `next-web-mq-*` | MQTT, Kafka, RabbitMQ, RocketMQ, and other messaging integrations |
| `next-web-mail` | Mail sending and MIME message construction |
| `next-web-xxl-job` | XXL-JOB executor integration |
| `next-web-state-machine` | State machine modeling and runtime support |
| `next-web-retry` | Retry templates and `#[retryable]` |
| `next-web-actuator` | Health checks, environment information, thread, and system metrics |
| `next-web-gateway` | Gateway capabilities based on Pingora |
| `next-web-ai` | AI / DeepSeek model integrations |
| `next-web-document` | Excel, CSV, PDF, and other document parsing and generation |
| `next-web-pay-*` / `next-web-wechat` / `next-web-sms` | Payment, WeChat, SMS, and other business integrations |


## Feature Flags

`next-web` enables `decrypt-properties` by default. Other capabilities are opt-in:

| Feature | Description |
| --- | --- |
| `full` | Enables a combination of commonly used optional capabilities |
| `tls-rustls` | TLS support based on rustls |
| `enable-web-security` | Enables the security module |
| `enable-websocket` | Enables WebSocket support |
| `enable-scheduling` | Enables scheduled tasks |
| `enable-retry` | Enables retry support |
| `enable-i18n` | Enables internationalization |
| `enable-open-api` | Enables OpenAPI / Swagger documentation |
| `enable-prometheus` | Enables Prometheus metrics |
| `enable-state-machine` | Enables the state machine |
| `enable-thread-pool` | Enables thread pool support |
| `embed-resources` | Embeds resources into the binary |
| `embed-resources-debug` | Embeds resources in debug builds too |
| `digester` | Enables digest algorithm support |
| `decrypt-properties` | Enables configuration decryption (enabled by default) |
| `trace-log` | Track special logs |

## Examples

The `examples` directory contains runnable examples:

| Example | Command | Description |
| --- | --- | --- |
| Basic application | `cargo run -p next-web-examples --bin application` | Routing, singletons, and basic application structure |
| Configuration properties | `cargo run -p next-web-examples --bin properties` | `#[configuration_properties]` and configuration binding |
| Auto-configuration | `cargo run -p next-web-examples --bin auto_configuration` | `AutoConfiguration` and conditional wiring |
| Application events | `cargo run -p next-web-examples --bin application_event` | Application event listeners and publishing |
| Lifecycle | `cargo run -p next-web-examples --bin application_lifecycle` | Application startup and shutdown lifecycle |
| Scheduling | `cargo run -p next-web-examples --bin scheduled` | `#[scheduled]` and task registration |
| Decoding and validation | `cargo run -p next-web-examples --bin decoder_and_vaild` | `Data`, `Validated`, and validators |
| Internationalization | `cargo run -p next-web-examples --bin i18n` | `MessageSource` and localized messages |
| Security | `cargo run -p next-web-examples --bin security` | Integrating the security module |
| WebSocket | `cargo run -p next-web-examples --bin ws` | Registering WebSocket handlers |
| Redis | `cargo run -p next-web-examples --bin redis` | Redis templates and listeners |
| Database | `cargo run -p next-web-examples --bin database` | Database services and transactions |
| Retry | `cargo run -p next-web-examples --bin retry` | Retry templates and `#[retryable]` |
| State machine | `cargo run -p next-web-examples --bin state_machine` | State machine configuration and runtime |
| OpenAPI | `cargo run -p next-web-examples --bin open_api` | API documentation generation |

More examples — mail, MQTT, XXL-JOB, payments, AI, streaming responses, and others — are available in the `examples/` directory.

## Status and Feedback

The project is in the `0.x` stage, and its APIs are still evolving. Please use [Issues](https://github.com/yuenxillar/next-web/issues) to report problems, suggest use cases, or contribute.

## License

Licensed under MIT OR Apache-2.0. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE) for details.
