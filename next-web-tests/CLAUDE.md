# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This crate (`next-web-tests`) is the integration test and example suite for the **next-web** ecosystem — a Rust web framework inspired by Spring Boot. It exercises every sub-crate in the workspace. It lives inside a larger Cargo workspace at `../`.

## Build & Run Commands

```bash
# Build the main test application
cargo build

# Run the main test application (starts a web server on configured port)
cargo run

# Run a specific example (each example is its own binary)
cargo run --example test_pay
cargo run --example test_security
cargo run --example test_api_doc

# Run with API doc generation enabled
cargo run --features enable-api-doc

# Run tests
cargo test

# Run a single test
cargo test test_local_file_encrypt

# Type-check only (fast)
cargo check
```

## Architecture

### Application lifecycle

Every binary (main + examples) follows the same pattern:

1. Define a struct implementing the `Application` trait (from `next_web::application::Application`).
2. Override lifecycle hooks: `init_middleware()`, `on_ready()`, `application_router()`.
3. Mark `main()` with `#[tokio::main]` and `#[next_application]`.
4. Call `YourApp::run().await`.

The `#[next_application]` macro wires up the framework: it reads `application.yaml`, initializes the `ApplicationContext`, registers auto-configurations, and starts the HTTP server.

### Request routing

Routes are defined via attribute macros on functions or impl blocks. The macros come from `next_web::macros::bind`:
- `#[request_mapping(method = "...", path = "...")]` — any HTTP method
- `#[get_mapping(path = "...")]`, `#[post_mapping(path = "...")]`
- `#[any_mapping(path = "...", headers = [...], consume = "...", produce = "...")]` — with header/content-type matching

Routes can also be defined inside `impl` blocks to group them under a common path prefix.

### Dependency injection / singleton store

The `ApplicationContext` holds a type-erased singleton store. Key patterns:
- **Registration**: `ctx.insert_singleton_with_name(value, "name")` or `ctx.insert_singleton_with_default_name(value)` in `on_ready()`.
- **Extraction**: Use `FindSingleton<T>` as an axum extractor parameter. The framework resolves by type and optionally by a `#[find]` attribute for named lookups.
- **Auto-configuration**: Types implementing `AutoConfiguration` (analogous to Spring's `@Configuration`) can be discovered and run at startup. The `#[auto_configuration]` macro on an `impl` block scans `#[provider]` methods — these become singleton factories that support dependency injection via `#[autowired]` and conditional creation via `#[conditional_on_property]`.

### Idempotency

The `#[idempotency]` macro (placed above a route handler) enables idempotent request processing. It requires an `IdempotencyStore` singleton (e.g., `MemoryIdempotencyStore`) registered in the context. Parameters: `name` (store name), `key` (header name for the idempotency key), `cache_key_prefix`, `ttl`.

### Configuration

`resources/application.yaml` is the default configuration file. Properties use dot-separated keys (e.g., `next.server.port`). Values support variable substitution like `${COMPUTERNAME}` and `${next.application.name}`.

### Internationalization (i18n)

Message resource bundles live in `resources/messages/` with locale-specific filenames (e.g., `messages_zh_CN.properties`). Configured via `next.messages.local` in the YAML config.

## Key workspace crates (dev-dependencies)

These are path dependencies this crate exercises:

| Crate | Purpose |
|-------|---------|
| `next-web` | Core framework (routing, application, macros) |
| `next-web-core` | Shared traits, context, filters, state |
| `next-web-ai` | AI/LLM integrations (DeepSeek) |
| `next-web-pay-alipay` | Alipay payment integration |
| `next-web-security` | Auth filters, security middleware |
| `next-web-websocket` | WebSocket support |
| `next-web-mqtt` | MQTT client |
| `next-web-data-redis` | Redis data store |
| `next-web-data-database` | Database integration |
| `next-web-xxl-job` | XXL-Job distributed task scheduler |
| `next-web-mail` | Email sending |
| `next-web-document` | Document generation (Excel export) |
| `next-web-retry` | Retry logic |
| `next-web-state-machine` | State machine workflows |
| `next-web-ip` | IP utilities |

## Code conventions

- Edition 2024, minimum Rust 1.92.0
- `unsafe_code = "forbid"` workspace-wide
- Each example file is a self-contained binary showing one feature in isolation
- Clone-based DI (singletons are wrapped in `Arc`, extracted by cloning)
