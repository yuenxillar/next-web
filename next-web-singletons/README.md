# next-web-singletons

A lightweight, dependency-free singleton registry and lazy factory for Rust.

`next-web-singletons` is the storage and lifecycle layer behind the Next Web
dependency injection system. It is deliberately small and framework-agnostic:
it has no external dependencies, no global state, and no async runtime
requirement, so it can be used on its own or embedded in another container.

## Features

- Named singletons keyed by `(type, name)`, so the same name may be used by
  different types.
- Lazy creation with `get_or_create`, `get_or_default`, and their mutable
  variants.
- Direct registry access with `register_singleton`, `get_singleton`,
  `get_singleton_mut`, and `get_singleton_or_insert`.
- Borrow-only and cloneable registration: `register_singleton` stores an
  instance for borrows, while `register_cloneable_singleton` also records how to
  produce owned copies.
- Type-wide listing through `ListableSingletonFactory`.
- Type-erased primitives (`Key`, `Type`, and `DynSingle`) for containers that
  need to store values without naming their concrete type.
- No external dependencies.

## Installation

```toml
[dependencies]
next-web-singletons = "0.1"
```

## Quick Start

```rust
use next_web_singletons::factory::support::DefaultListableSingletonFactory;
use next_web_singletons::factory::{ListableSingletonFactory, SingletonFactory};

#[derive(Clone, Debug, PartialEq, Eq)]
struct AppConfig {
    name: String,
}

fn main() {
    let mut factory = DefaultListableSingletonFactory::new();

    let config: &AppConfig = factory.get_or_create("config", || AppConfig {
        name: "next-web".to_owned(),
    });
    assert_eq!(config.name, "next-web");

    // The factory closure is not called again for an existing singleton.
    let config: &AppConfig = factory.get_or_create("config", || AppConfig {
        name: "ignored".to_owned(),
    });
    assert_eq!(config.name, "next-web");

    // Every singleton of a type can be listed by a listable factory.
    let configs: Vec<&AppConfig> = factory.get_singletons_of_type::<AppConfig>();
    assert_eq!(configs.len(), 1);

    let names: Vec<&str> = factory.get_singleton_names_for_type::<AppConfig>();
    assert_eq!(names, vec!["config"]);
}
```

## Registry Example

```rust
use next_web_singletons::factory::config::SingletonRegistry;
use next_web_singletons::factory::support::DefaultSingletonRegistry;

#[derive(Clone, Debug, PartialEq, Eq)]
struct CacheConfig {
    max_size: usize,
}

fn main() {
    let mut registry = DefaultSingletonRegistry::default();

    registry.register_singleton("cache", CacheConfig { max_size: 1024 });
    assert!(registry.contains_singleton::<CacheConfig, _>("cache"));

    let cache = registry
        .get_singleton::<CacheConfig, _>("cache")
        .expect("the cache config is registered");
    assert_eq!(cache.max_size, 1024);

    // `register_cloneable_singleton` records how to produce an owned copy.
    registry.register_cloneable_singleton("cloneable", CacheConfig { max_size: 2048 });
    let owned = registry
        .get_singleton_owned::<CacheConfig, _>("cloneable")
        .expect("the cloneable config is registered");
    assert_eq!(owned.max_size, 2048);
}
```

## Core Types

| Type | Responsibility |
| --- | --- |
| `SingletonRegistry` | Stores named instances and provides register/get/remove operations. |
| `SingletonFactory` | Adds lazy creation, defaults, removal, listing, and counters on top of a registry. |
| `ListableSingletonFactory` | Lists every instance and every name of a given type. |
| `DefaultSingletonRegistry` | The default `HashMap`-backed registry. |
| `DefaultListableSingletonFactory` | The default factory, backed by `DefaultSingletonRegistry`. |
| `Key` | Pairs the `TypeId` and the name that identify a stored instance. |
| `Type` | A type descriptor holding a `TypeId` and a readable type name. |
| `DynSingle` | A type-erased stored instance, with an optional clone function. |

## Design Notes

- A key is `(TypeId, name)`. Registering the same name for two types creates
  two independent entries.
- `DefaultSingletonRegistry` stores its entries in a `HashMap`, so the order of
  `get_singleton_names` and `get_all_singleton_names` is not specified.
- `register_singleton` and `get_singleton_or_insert` store borrow-only
  instances. `get_singleton_owned` returns `None` unless the instance was
  registered through `register_cloneable_singleton`.
- A factory owns its registry. There is no global or static container.
- The crate has no async runtime dependency. Async construction belongs to the
  provider layer of `next-web-context`.

## Role in Next Web

`next-web-context` uses `next-web-singletons` as the registry behind its
`ApplicationContext` and provider APIs. Using the registry directly is useful
when a library needs the same named-singleton semantics without depending on
the full application context.

## License

Part of the Next Web project, licensed under MIT OR Apache-2.0.
