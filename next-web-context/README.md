# next-web-context

The application context, dependency injection, event, and message-source layer
of [Next Web](https://github.com/yuenxillar/next-web).

`next-web-context` is the runtime seam between the Next Web framework and the
libraries that plug into it. It deliberately depends on a small set of
infrastructure crates rather than on the web server or HTTP client, so a library
can contribute providers, listeners, or message sources without pulling in the
full framework.

## What Is Inside

- `ApplicationContext`: the object-safe service provider interface for
  type-erased singleton operations.
- `ApplicationContextExt`: the generic, type-safe API used by generated code and
  application code.
- `Provider`: the dependency injection model with singleton, transient, and
  single-owner scopes, synchronous and asynchronous constructors, module
  composition, eager creation, binding providers, and conditions.
- Auto registration: `register_provider!`, `AutoRegisterModule`, and `enable!`
  let a crate submit its providers without the application listing them.
- Events: `ApplicationEvent`, `ApplicationListener`, `ApplicationEventPublisher`,
  typed/generic/smart listeners, and the default event multicaster.
- i18n: `Locale`, `LocaleContext`, `MessageSource`, `MessageSourceResolvable`,
  `NoSuchMessageError`, and the resource-based message sources.
- Support utilities: `MessageSourceAccessor`, `DelegatingMessageSource`,
  `PropertySourcesPlaceholderConfigurer`, `Properties`, and `MessageFormat`.

## Installation

```toml
[dependencies]
next-web-context = "0.1"
```

## Application Context

`ApplicationContext` is object safe and works with type-erased instances, which
is what allows providers and extension points to receive
`&mut dyn ApplicationContext` without depending on a concrete context type.

The generic helpers live in `ApplicationContextExt` and follow a consistent
naming convention:

- no suffix: the instance is registered under the empty name `""`,
- `with_name`: the instance is registered under the given name,
- `with_default_name`: the instance is registered under the name derived from
  its type.

```rust
use next_web_context::{ApplicationContext, ApplicationContextExt};

fn configure(context: &mut dyn ApplicationContext) {
    if let Some(name) = context.get_singleton_option_with_default_name::<String>() {
        println!("application name: {name}");
    }
}
```

## Providers and Scopes

Providers describe how an instance is created, which name it is registered
under, and when it has to exist. A provider constructor receives
`&mut dyn ApplicationContext`, so it can resolve its own dependencies through
the same generic API.

```rust
use next_web_context::{Provider, singleton};

#[derive(Clone, Debug)]
struct Greeting(String);

fn greeting_provider() -> Provider<Greeting> {
    singleton(|_| Greeting("Hello".to_owned()))
        .name("greeting")
        .into()
}
```

| Constructor | Scope | Notes |
| --- | --- | --- |
| `singleton` | `Singleton` | One instance; the type must implement `Clone`. |
| `transient` | `Transient` | A new instance is created for every resolution. |
| `single_owner` | `SingleOwner` | One instance; only borrowed access is exposed. |
| `singleton_async` | `Singleton` | Async variant of `singleton`. |
| `transient_async` | `Transient` | Async variant of `transient`. |
| `single_owner_async` | `SingleOwner` | Async variant of `single_owner`. |

Providers can also use `bind` to expose the same instance as another type, set
a condition with `condition`, request eager creation with `eager_create`, or be
grouped into a `Module`.

## Events

The event layer contains the usual Next Web/Spring-style building blocks:

- `ApplicationEvent` for event payloads,
- `ApplicationListener`, `TypedApplicationListener`, `GenericApplicationListener`,
  and `SmartApplicationListener` for listeners,
- `ApplicationEventPublisher` for publishing,
- `ApplicationEventMulticaster` and `DefaultApplicationEventMulticaster` for
  dispatch, with configurable dispatch modes and listener failure policies.

## i18n and Message Sources

The i18n layer models locales, locale context, and message resolution:

- `Locale` for language, country, and variant handling,
- `LocaleContext` and `LocaleContextHolder` for the current locale,
- `MessageSource` and `MessageSourceResolvable` for message lookup,
- `ResourceBundleMessageSource` for locale-specific properties bundles,
- `MessageSourceAccessor` for convenient access,
- `DelegatingMessageSource` for parent delegation.

The utility layer also provides message rendering and property handling:

```rust
use next_web_context::util::MessageFormat;

let format = MessageFormat::new("Hello {0}, you are {1}.");
assert_eq!(format.format(&[&"Rust", &2024]), "Hello Rust, you are 2024.");
```

```rust
use next_web_context::Locale;

let locale = Locale::for_language_tag("zh-CN").unwrap();
assert_eq!(locale.language(), "zh");
assert_eq!(locale.country(), Some("CN"));
assert_eq!(locale.to_bundle_suffix(), "zh_CN");
```

## Module Map

| Module | Contents |
| --- | --- |
| `provider` | Application context SPI, type-safe helpers, providers, scopes, modules, constructors, and conditions. |
| `autoregister` | Inventory-based provider submission and `AutoRegisterModule`. |
| `event` | Event publishers, listeners, adapters, multicasters, and failure policies. |
| `i18n` | Locale context types. |
| `support` | Message source implementations, message accessor, delegating source, and property placeholder configurer. |
| `util` | `Locale`, `MessageFormat`, and `Properties`. |

## Relationship to Next Web

`next-web` provides the concrete application context, environment, web server,
and runtime integration. `next-web-context` is the lightweight layer that
third-party crates and generated code can depend on when they only need to
contribute providers, events, or messages.

## Status

The crate is in the `0.x` stage and its API is still evolving. Synchronous
providers are the stable path today; asynchronous provider creation is part of
the provider model and is being completed together with the concrete contexts.

## License

Part of the Next Web project, licensed under MIT OR Apache-2.0.
