extern crate proc_macro;

use crate::data::application_event::impl_macro_application_event;
use crate::data::builder::impl_macro_builder;
use crate::util::logic::Logic;
use crate::data::constructor::impl_macro_required_args_constructor;
use crate::data::field_name::impl_macro_field_name;
use crate::data::get_set::impl_macro_get_set;
use crate::web::configuration_properties::impl_macro_configuration_properties;
use crate::web::idempotency::impl_macro_idempotency;
use crate::web::order::generate_order;
use crate::web::post_authorize::impl_macro_post_authorize;
use crate::web::pre_authorize::impl_macro_pre_authorize;

use proc_macro::TokenStream;
use syn::parse_macro_input;
use syn::DeriveInput;
use syn::ItemFn;
use syn::ItemImpl;
use syn::ItemStruct;

use next_web_context::Scope;

mod data;
mod di;
mod util;
mod web;
mod state_machine;

#[cfg(feature = "resilience")]
mod resilience;

#[cfg(feature = "resilience")]
use crate::resilience::{Pattern, apply};

/// Automatically generate getter and setter methods for structures.
///
/// This macro will generate corresponding 'get_ {field}' and 'set_ {field}' methods for each field of the structure,
/// Unless the '# [get_det (...)' attribute is used on the field for skip control.
///
/// # Field attribute control
///
/// - ` # [get_det (skip)] `: Skip the generator and setter generation for this field.
/// - ` # [get_det (skip_get)] `: Only skip the generation of the getter method.
/// - ` # [get_det (skip_set)] `: Skip only the generation of setter methods.
///
/// # Example
///
/// ```rust
/// # use next_web_macros::GetSet;
/// #[derive(GetSet)]
/// struct MyStruct {
///     #[get_set(skip)]
///     field1: i32, //Do not generate any methods
///     field2: i32, //Generate get_field2() and set_field2()
///     #[get_set(skip_get)]
///     field3: i32, //Only generate set_field3()
///     #[get_set(skip_set)]
///     field4: i32, //Only generate get_field4()
/// }
/// ```
#[proc_macro_derive(GetSet, attributes(get_set))]
pub fn get_set(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    impl_macro_get_set(&input)
}

/// Method for generating corresponding field names for each field of a structure (string literal)
///
/// The method name is' field_ {field_name} ', the return type is' static str' (such as' field_name '), and the value is the original name of the field (such as' name')
///
/// # Example
///
/// ```ignore
/// # use next_web_macros::FieldName;
/// #[derive(FieldName)]
/// struct Person {
///     name: String,
///     age: u32,
/// }
///
/// impl Person {
///
///     pub fn field_name() -> &'static str {
///         "name"
///     }
///
///     pub fn field_age() -> &'static str {
///         "age"
///     }
/// }
/// ```
#[proc_macro_derive(FieldName)]
pub fn field_name(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    impl_macro_field_name(&input)
}

/// Implements `ApplicationEvent` for the annotated value.
///
/// Every event of the application implements the trait, and the three methods of
/// it that are the same for every event are boilerplate that the derive writes:
/// the type of the event and the type of its source are the type of the value, and
/// the source of the event is the value itself. A listener that reads the source
/// downcasts the event, and the filter of the event system matches the value with
/// the type it reports, so the two cannot disagree.
///
/// The annotated value has to be `Clone` and `'static`: every event of the system
/// is cloneable, and the type of an event is a `TypeId`.
///
/// The derive does not implement `timestamp`, because a generated method would
/// report the time it is called rather than the time the event happened. An event
/// that carries the time it happened keeps it in a field.
///
/// # Example
///
/// ```ignore
/// use next_web_context::ApplicationEvent;
///
/// #[derive(Clone, ApplicationEvent)]
/// struct OrderPlaced {
///     id: String,
/// }
/// ```
///
/// The trait is named as `::next_web_context::ApplicationEvent`, so the crate that
/// annotates a value depends on `next-web-context`.
#[proc_macro_derive(ApplicationEvent)]
pub fn application_event(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    Logic::generate(|| impl_macro_application_event(&input))
}

/// Implement the Builder pattern for structures
///
/// This macro will generate a 'Builder' struct with the same name and provide a chain call setter method for each field
/// Finally, the 'build()' method is used to construct the original structure instance
///
/// ## Attention
///
/// 1. By default, even if the field is Option, the user still needs to assign a value to this field before the build method, unless the default auxiliary parameter is used
///
/// Support customizing field behavior through the '# [builder (...)]' attribute<br>
/// Auxiliary parameters:<br>
/// Into: The input parameter of the method generated by the boiler will be a type that implements Into<field><br>
/// ` default `:<br>1. In the build method, if there is no user input for this field in the builder structure, Default:: default() will be used to assign it to the original instance<br>
///                   2. If #\[builder (default="default_name")\], the return value of this function will be used for assignment
///
/// # Example
///
/// ```ignore
/// # use next_web_macros::Builder;
/// #[derive(Builder)]
/// struct User {
///     #[builder(into)]
///     name: String,
///     age: u32,
/// }
///
/// // Generate UserBuilder structure
/// struct UserBuilder {
///     name: Option<String>,
///     age: Option<u32>,
/// }
///
///
/// let user = User::builder()
///     .name("Alice")
///     .age(30)
///     .build()
///     .unwrap();
/// ```
#[proc_macro_derive(Builder, attributes(builder))]
pub fn builder(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    impl_macro_builder(&input)
}

/// Implement constructor with required arguments for structures
///
/// This macro will generate a default constructor and provide corresponding parameters for each field,
/// Finally, the parameters are used to construct the original structure instance
///
/// ## Attention
///
/// 1. If the field is an Option type, it will not appear in the parameter list unless the required auxiliary parameter is used
///
/// Auxiliary parameters:
///
/// `required`: If the field is an Option type, it will appear in the method parameter list
/// `default`: If there is no parameter for this field, use Default::default() to assign it
/// `into`: The parameter will be converted to a type that implements Into<field>
///
/// # Example
///
/// ```ignore
/// #[derive(RequiredArgsConstructor)]
/// struct Person {
///     #[constructor(into)]
///     name: String,
///     age: u32,
/// }
///
/// let person = Person::from_args("Alice", 30);
/// ```
#[proc_macro_derive(RequiredArgsConstructor, attributes(constructor))]
pub fn required_args_constructor(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    impl_macro_required_args_constructor(&input)
}

/// Implements `desensitize` for a structure, which masks the fields that name a
/// desensitizer with `#[de]`.
///
/// A field names either a type the framework provides, or a function of the
/// application that takes `&str` and answers the value of the field:
///
/// ```ignore
/// #[derive(Desensitize, Serialize)]
/// struct Person {
///     #[de(phone)]
///     phone: String,
///
///     #[de(email)]
///     email: Option<String>,
///
///     #[de(with = de_plate)]
///     plate: Cow<'static, str>,
/// }
///
/// /// Masks every character of a plate but the first two.
/// fn de_plate(value: &str) -> String {
///     value.chars().take(2).collect::<String>() + "***"
/// }
/// ```
///
/// # The `de` attribute
///
/// A field names one desensitizer, and the two forms are mutually exclusive:
///
/// * **a type of the framework**, named on its own: `#[de(phone)]`. The types
///   are `email`, `phone`, `password`, `name`, `id_card`, `bank_card`,
///   `address`, and `ip`, and each of them is the function of
///   [`DesensitizedUtil`] of the same name;
/// * **a function of the application**, named with `with`:
///   `#[de(with = de_phone)]`, or by its path, as in
///   `#[de(my_module::de_phone)]`. The function takes `&str` and answers the
///   value of the field.
///
/// An attribute that names both, a second `de` attribute on the same field, and
/// a `de` attribute on the field of a type that is not text (`String`,
/// `Box<str>`, `Cow<str>`, or one of them in an `Option`) are reported as errors
/// of the derive.
///
/// [`DesensitizedUtil`]: next_web::util::desensitized::DesensitizedUtil
#[proc_macro_derive(Desensitize, attributes(de))]
pub fn desensitize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    data::desensitize::impl_macro_desensitize(&input)
}

// =============================== Web ===============================

///
/// Process macros used to extend the entry point of Next web applications
///
/// # Attributes
/// * `resources` - Optional parameter specifying the folder containing embedded resources
///                 Example: `#[next_application(resources = "static")]`
///
/// # Behavior
/// - Transforms a function into a Next-web application entry point
/// - If `resources` attribute is provided, enables resource embedding from the specified folder
///
/// # Example
/// ```ignore
/// use next_web::macros::application::next_application;
///
/// /// Basic application without embedded resources
/// #[tokio::main]
/// #[next_application]
/// async fn main() {
///     // Application code here
/// }
///
/// /// Application with embedded resources from "static/" folder
/// #[tokio::main]
/// #[next_application(resources = "static/")]
/// async fn main() {
///     // Access embedded resources via ApplicationResources
/// }
/// ```
#[proc_macro_attribute]
pub fn next_application(attr: TokenStream, item: TokenStream) -> TokenStream {
    use crate::web::application::impl_macro_application;

    let item_fn = parse_macro_input!(item as ItemFn);
    impl_macro_application(attr, item_fn)
}

/// Declares an `impl` block as a configuration class of the application.
///
/// A method of the block that declares a provider is called while the
/// application starts: it resolves the parameters of the method from the
/// application context, and registers the value it returns as a singleton. The
/// configuration class itself is an
/// [`AutoConfiguration`](next_web_core::traits::config::auto_configuration::AutoConfiguration),
/// so the framework applies it together with the auto-configurations of the
/// starters, in the order of [`Ordered`](next_web_core::Ordered), which the
/// `before` and `after` names a configuration declares refine.
///
/// # Arguments
///
/// - `order` - The order the configuration is applied in. It defaults to `100`,
///   the default of [`Ordered`](next_web_core::Ordered), and it places the
///   configuration among the configurations of its own phase: the
///   configurations of an application are applied before the
///   auto-configurations of the starters, whatever order the two declare.
/// - `conditional = [path, ...]` - The conditions the configuration is applied
///   under. Each names a function that takes `&dyn ApplicationContext` and
///   returns a `bool`, and the configuration is skipped when one of them
///   returns `false`.
/// - `before = ["...", ...]` - The names of the configurations this one is
///   applied before. A configuration is named after its type, so
///   `before = ["RedisAutoConfiguration"]` applies the configuration before the
///   auto-configuration of the Redis starter, even when the order it declares is
///   the higher one. A name no configuration of the application provides is
///   ignored.
/// - `after = ["...", ...]` - The names of the configurations this one is
///   applied after, which is the counterpart of `before`.
/// - `name = "..."` - The name the configuration is registered under. It
///   defaults to the name of the type the macro generates.
///
/// # Method attributes
///
/// - `#[provider]`, which is required for a method to be called, takes
///   `name = "..."`, `conditional = [path, ...]` and `order = <integer>`. A
///   provider runs in the order of its `order`, which defaults to `100`, and
///   the providers of the same order run in the order they are declared in.
/// - `#[autowired(name = "...", default)]` of a parameter resolves the
///   parameter from the context by name, and falls back to `Default::default()`
///   when the name is not registered.
/// - A parameter declared as a reference, `&T` or `&mut T`, borrows the
///   instance the context holds instead of resolving a copy of it, so the
///   provider sees the instance the other providers see. A method can take any
///   number of `&T` parameters, or a single `&mut T` one, but a `&mut T`
///   parameter cannot be combined with another reference parameter, because the
///   context lends its instances while it is borrowed. `#[autowired(default)]`
///   is only supported on a parameter the context hands out by value, because
///   the fallback would be a temporary value rather than an instance of the
///   context.
/// - `#[conditional_on_property(name = "...", having_value = "...")]` creates
///   the instance only when the property holds the given value.
///
/// The methods of the block take their dependencies as parameters and do not
/// take `self`, and a method that returns a `Result` reports its failure to the
/// application, which stops the startup.
///
/// # Example
///
/// ```ignore
/// use next_web::macros::autoconfigure::auto_configuration;
///
/// #[auto_configuration(
///     order = 200,
///     after = ["RedisAutoConfiguration"],
///     conditional = [Self::feature_enabled]
/// )]
/// impl MyConfiguration {
///     #[provider(name = "myClient", order = 10)]
///     fn my_client(#[autowired(name = "myProperties")] properties: MyProperties) -> Client {
///         Client::with_properties(&properties)
///     }
///
///     fn feature_enabled(ctx: &dyn ApplicationContext) -> bool {
///         ctx.contains_singleton::<Client>()
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn auto_configuration(attr: TokenStream, item: TokenStream) -> TokenStream {
    use crate::web::auto_configuration::impl_macro_auto_configuration;

    let item_impl = parse_macro_input!(item as ItemImpl);
    impl_macro_auto_configuration(attr, item_impl)
}

/// Declares an `impl` block as the configuration class of an application.
///
/// A configuration class is applied **before** the auto-configurations of the
/// application, so that it can contribute the state an auto-configuration of a
/// starter depends on. A method that declares a provider is called while the
/// application starts: it resolves the parameters of the method from the
/// application context, and registers the value it returns as a singleton. A
/// method declared with `#[init]` is an initializer: it resolves its parameters
/// the same way, runs without registering a value, and does not return a value.
///
/// The configuration class itself implements
/// [`Configuration`](next_web_core::traits::config::configuration::Configuration),
/// and its `before` and `after` names place it among the configurations of the
/// application.
///
/// # Arguments
///
/// - `order` - The order the configuration is applied in. It defaults to `100`,
///   the default of [`Ordered`](next_web_core::Ordered), and it places the
///   configuration among the configurations of its own phase: the
///   configurations of an application are applied before the
///   auto-configurations of the starters, whatever order the two declare.
/// - `conditional = [path, ...]` - The conditions the configuration is applied
///   under. Each names a function that takes `&dyn ApplicationContext` and
///   returns a `bool`, and the configuration is skipped when one of them
///   returns `false`.
/// - `before = ["...", ...]` - The names of the configurations this one is
///   applied before, which is the counterpart of `after`.
/// - `after = ["...", ...]` - The names of the configurations this one is
///   applied after.
/// - `name = "..."` - The name the configuration is registered under. It
///   defaults to the name of the type the macro generates.
///
/// # Method attributes
///
/// - `#[provider]`, which is required for a method to register a value, takes
///   `name = "..."`, `conditional = [path, ...]` and `order = <integer>`.
/// - `#[init]` declares an initializer. It takes `conditional = [path, ...]` and
///   `order = <integer>`, does not return a value, and runs next to the
///   providers and the initializers of the same order.
/// - `#[autowired(name = "...", default)]` of a parameter resolves the parameter
///   from the context by name, and falls back to `Default::default()` when the
///   name is not registered.
/// - A parameter declared as a reference, `&T` or `&mut T`, borrows the instance
///   the context holds instead of resolving a copy of it. A method can take any
///   number of `&T` parameters, or a single `&mut T` one, but a `&mut T`
///   parameter cannot be combined with another reference parameter.
///
/// # Example
///
/// ```ignore
/// use next_web::macros::autoconfigure::configuration;
///
/// #[configuration]
/// impl MyConfiguration {
///     #[provider(name = "myClient")]
///     fn my_client(properties: MyProperties) -> Client {
///         Client::with_properties(&properties)
///     }
///
///     #[init]
///     fn register(registry: &mut ServiceRegistry, client: &Client) {
///         registry.register(client);
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn configuration(attr: TokenStream, item: TokenStream) -> TokenStream {
    use crate::web::auto_configuration::impl_macro_configuration;

    let item_impl = parse_macro_input!(item as ItemImpl);
    impl_macro_configuration(attr, item_impl)
}

/// Declares the configuration properties of a struct.
///
/// The properties below the prefix are bound from the environment of the
/// application, and the bound instance is registered as a singleton under the
/// name of the struct. The properties are registered before the
/// auto-configurations of the application run, so an auto-configuration that
/// depends on them can resolve them.
///
/// The name of a field is the name of the property it is bound from, which the
/// `#[key = "..."]` attribute of the field overrides. A field is bound from an
/// environment variable as well, which is named after the property.
///
/// The struct has to derive `Clone`, `Default` and `Deserialize`. It is
/// normally declared as a `#[singleton]` as well, which registers the type
/// itself; the properties are registered whether or not it is. With `dynamic`,
/// the properties that a key below the prefix groups are bound as well, and can
/// be read with the generated `dynamic_properties` method.
///
/// # Example
///
/// ```ignore
/// #[singleton(default, binds = [Self::into_properties])]
/// #[configuration_properties(prefix = "next.data.redis", dynamic)]
/// #[derive(Debug, Clone, Default, serde::Deserialize)]
/// pub struct RedisProperties {
///     host: Option<String>,
///     port: Option<u16>,
/// }
/// ```
#[proc_macro_attribute]
pub fn configuration_properties(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ItemStruct);
    impl_macro_configuration_properties(attr, item)
}

/// Registers the annotated function as a route of the application.
///
/// The macro turns the function into a handler that the router picks up while
/// the application starts, so the function itself is not called directly. It
/// has to be `async`, must not take `self` and has to return a response; every
/// other argument is treated as an axum extractor and is resolved from the
/// request. The declared return type is replaced by `impl IntoResponse`, while a
/// `Result<_, _>` type is kept on the body so its error type stays inferable.
///
/// The macro can also be applied to an `impl` block that holds `#[*_mapping]`
/// methods. The `path` of the block is then prepended to the `path` of every
/// annotated method, and those methods are hoisted out of the block as ordinary
/// handlers.
///
/// # Attributes
///
/// * `path` - the route pattern, for example `"/orders/{id}"`. Required.
/// * `method` - the HTTP method the route answers, one of `GET`, `POST`, `PUT`,
///   `DELETE`, `HEAD`, `CONNECT`, `OPTIONS`, `TRACE`, `PATCH` or `ANY`. Required
///   here, because `request_mapping` is the macro without an implied method; the
///   per-method macros set it themselves and reject it. It cannot be used on an
///   `impl` block.
/// * `headers` - names of headers a request has to carry, for example
///   `["Authorization"]`. Each name adds a
///   `RequiredHeader<header_names::Name>` extractor argument, so a request that
///   misses one is answered with `400 Bad Request` before the body runs.
/// * `consume` - the media type expected in the request. A request whose
///   `Content-Type` does not contain it is answered with
///   `415 Unsupported Media Type`.
/// * `produce` - the media type of the response. It is written to the
///   `Content-Type` header of whatever the handler returns.
///
/// # Ordering
///
/// Attributes that wrap a handler, such as [`pre_authorize`] or
/// `#[idempotency]`, have to be placed *above* the routing attribute. The
/// routing macro rebuilds the function and only re-emits the attributes it is
/// given, so an attribute placed below it is dropped.
///
/// # Example
///
/// ```ignore
/// use next_web::macros::bind::{get_mapping, request_mapping};
///
/// #[request_mapping(method = "POST", path = "/orders", consume = "application/json")]
/// async fn create_order(order: axum::Json<Order>) -> impl IntoResponse {
///     // ...
/// }
///
/// #[request_mapping(path = "/admin")]
/// impl AdminApi {
///     #[get_mapping(path = "/users")]
///     async fn users() -> &'static str {
///         "[]"
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn request_mapping(args: TokenStream, input: TokenStream) -> TokenStream {
    crate::web::routing::with_method(None, args, input)
}

/// Generates one routing attribute macro per HTTP method.
///
/// Each invocation expands to a `#[proc_macro_attribute]` that forwards to
/// [`crate::web::routing::with_method`] with the method baked in. The generated
/// macros therefore share the options and the behaviour of [`request_mapping`];
/// they differ from it only in that `method` must not be given, because the name
/// of the macro already selects the method.
macro_rules! http_method_macro {
    ($(#[$meta:meta])* $method:ident, $variant:ident, $http:literal) => {
        /// Registers the annotated function as a route of the application.
        ///
        #[doc = concat!(
            "The route answers HTTP `", $http, "` requests, that is, this macro is ",
            "[`request_mapping`] with `method = \"", $http, "\"` already applied. The ",
            "`path` (required), `headers`, `consume` and `produce` attributes behave ",
            "exactly as described there, and `method` must not be given again."
        )]
        $(#[$meta])*
        #[proc_macro_attribute]
        pub fn $method(args: TokenStream, input: TokenStream) -> TokenStream {
            crate::web::routing::with_method(
                Some(crate::web::routing::Method::$variant),
                args,
                input,
            )
        }
    };
}

http_method_macro! {
    /// # Example
    ///
    /// ```ignore
    /// use next_web::macros::bind::get_mapping;
    ///
    /// #[get_mapping(path = "/hello")]
    /// async fn hello() -> &'static str {
    ///     "Hello!"
    /// }
    /// ```
    get_mapping, Get, "GET"
}

http_method_macro! {
    post_mapping, Post, "POST"
}

http_method_macro! {
    put_mapping, Put, "PUT"
}

http_method_macro! {
    delete_mapping, Delete, "DELETE"
}

http_method_macro! {
    patch_mapping, Patch, "PATCH"
}

http_method_macro! {
    /// A route that answers any HTTP method.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use next_web::macros::bind::any_mapping;
    ///
    /// #[any_mapping(path = "/webhook")]
    /// async fn webhook() -> &'static str {
    ///     "ok"
    /// }
    /// ```
    any_mapping, Any, "ANY"
}

#[cfg(feature = "open-api")]
#[proc_macro_attribute]
pub fn open_api(args: TokenStream, input: TokenStream) -> TokenStream {
    crate::web::open_api::impl_macro_open_api(args, input)
}

/// Declares a function as a task of the scheduler of the application.
///
/// The function is registered while the application starts, without the
/// application listing it, and it is persisted with its schedule, so that the
/// state of the task (whether it is enabled, when it last ran) is kept by the
/// repository of the application. See the `scheduling` module of `next-web` for
/// the pieces the macro generates and for what the state is worth.
///
/// A task takes its dependencies as its parameters, which are resolved from the
/// application context, so `fn cleanup(service: MyService)` is given the
/// instance registered under the default name of `MyService`.
///
/// It supports three scheduling modes:
///
/// - **Cron-based scheduling**: via the `cron` parameter (e.g., `"0 0 2 * * *"`).
/// - **Fixed-rate execution**: via the `fixed_rate` parameter (executes repeatedly at fixed intervals).
/// - **One-shot execution**: when `one_shot` is set, the task runs once, after `initial_delay`.
///
/// # Parameters
///
/// - `cron`: A cron expression in 6-field format (seconds, minutes, hours, day-of-month, month, day-of-week).
///   Mutually exclusive with `fixed_rate`.
/// - `fixed_rate`: Interval between executions (as a positive integer literal).
///   Mutually exclusive with `cron`.
/// - `initial_delay`: Delay before the first execution (in units specified by `time_unit`).
///   Only a `one_shot` task supports it.
/// - `timezone`: IANA time zone ID (e.g., `"Asia/Shanghai"`, `"UTC"`).
///   If empty or omitted, the scheduler's default time zone is used.
/// - `time_unit`: Time unit for `fixed_rate` and `initial_delay` (e.g., `"ms"`, `"s"`, `"m"`).
///   Milliseconds are the default, and the scheduler evaluates a schedule once
///   per second at the most, so a shorter duration is raised to one second.
/// - `one_shot`: If set, the task runs exactly once, after `initial_delay`.
/// - `name`: The key the task is registered and persisted under. It defaults to
///   the path of the function, and it is what keeps a persisted schedule of a
///   task from being declared again when the function moves.
///
/// The function may return nothing, or a `Result` whose failure the scheduler
/// logs while the task keeps its schedule.
///
/// # Examples
///
/// ```ignore
/// use next_web::macros::scheduled;
///
/// #[scheduled(cron = "0 0 3 * * *", timezone = "UTC")]
/// fn daily_cleanup() { /* ... */ }
///
/// #[scheduled(fixed_rate = 30, time_unit = "s", name = "heartbeat")]
/// async fn heartbeat(cluster: Cluster) { /* ... */ }
///
/// #[scheduled(one_shot, initial_delay = 10, time_unit = "s")]
/// fn delayed_init() { /* ... */ }
/// ```
#[proc_macro_attribute]
pub fn scheduled(args: TokenStream, input: TokenStream) -> TokenStream {
    crate::web::scheduled::impl_macro_scheduled(args, input)
}

///
/// Process macros for implementing idempotent properties
///
/// # Description of Attribute Parameters
/// - ` name `: optional string literal, specifying the singleton name of IdempotencyStore, can specify the name to use their own implemented Store, default is ` memoryIdempotencyStore '`
/// - ` key `: optional string literal, used to obtain the key value of the request header, default value is ` Idempotency key ``
/// - ` cache_key_prefix `: optional string literal, apply the prefix of the cached key again
/// - ` ttl `: optional integer face count, cache live time (Time To Live), default is seconds
///
/// # Attention
/// The current macro only supports Mehod as' POST '`
/// You should use PostMapping or RequestMapping (method="POST")`
///
/// # Example
/// ```ignore
/// #[idempotency(name = "myIdempotencyStore", key = "Idempotency-key", cache_key_prefix = "test", ttl = 6)]
/// #[PostMapping(path = "/createOrder")]
/// async fn create_order(order: String) -> impl IntoResponse {
///
///     "Ok"
/// }
///
/// ```
#[proc_macro_attribute]
pub fn idempotency(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_fn = parse_macro_input!(item as ItemFn);
    impl_macro_idempotency(attr, item_fn)
}

/// Restricts a handler to the callers that satisfy an access-control rule.
///
/// The annotated function has to be `async`, must not take `self` and has to
/// return a response. Before its body runs, the rule is evaluated against the
/// authentication held by the security context of the current request; when the
/// rule does not hold, the handler answers `403 Forbidden` and the body is
/// skipped. Like the routing macros, the annotation rewrites the return type to
/// `impl IntoResponse`.
///
/// Roles are compared with the `ROLE_` prefix of Spring Security, so
/// `has_role("ADMIN")` matches a granted authority called `ROLE_ADMIN`;
/// authorities are compared as they are.
///
/// The macro is available with the `enable-web-security` feature, which exposes
/// it as `next_web::macros::security::pre_authorize`.
///
/// # Expression form
///
/// `#[pre_authorize("...")]` takes a Spring Security style expression built from
/// the functions `has_role`, `has_any_role`, `has_authority`,
/// `has_any_authority`, `permit_all` and `deny_all`, and from the operators
/// `and` (`&&`), `or` (`||`) and `!` (`not`). Parentheses group sub-expressions,
/// `and` binds tighter than `or`, and the camel case spellings such as
/// `hasRole` are accepted as well.
///
/// # Structured form
///
/// The same rule can be built from key/value attributes:
///
/// * `has_role` - a single required role.
/// * `has_any_role` - a list of roles, any of which grants access.
/// * `has_authority` - a single required authority.
/// * `has_any_authority` - a list of authorities, any of which grants access.
/// * `role` / `permission` - lists of roles and authorities kept for
///   compatibility, combined like the checks above.
/// * `mode` - how the configured checks are combined, either `"And"` (the
///   default: every check has to hold) or `"Or"` (at least one has to hold).
/// * `negate` - inverts the whole rule when it is `true`, which is how "the
///   caller must *not* have this role" is written.
/// * `expr` - the same expression string as the positional form.
/// * `ignore` - skips the check and emits the handler unchanged.
/// * `basic` - reserved for HTTP basic authorization. It is rejected at compile
///   time, so a rule is never dropped silently.
///
/// # Ordering
///
/// Like the other wrapping handler attributes, `#[pre_authorize]` has to be
/// placed *above* the routing attribute. The routing macro only re-emits the
/// attributes it receives, so an inner `#[pre_authorize]` would be dropped and
/// the handler would be left unguarded.
///
/// # Refusal
///
/// The rule is evaluated against the authorities of the request, which the
/// crate that authenticates the requests writes into it. A request the rule
/// refuses is answered by the handler the application configured for a refused
/// request (`ErrorHandlingConfigurer::access_denied_handler`), and with `403
/// Forbidden` when the application configured none. The guard is the first
/// argument of the handler, so a handler that consumes the request body still
/// receives it last.
///
/// # Example
///
/// ```ignore
/// use next_web::macros::bind::get_mapping;
/// use next_web::macros::security::pre_authorize;
///
/// #[pre_authorize("has_role('ADMIN') and has_authority('order:read')")]
/// #[get_mapping(path = "/admin/orders")]
/// async fn orders() -> &'static str {
///     "[]"
/// }
///
/// #[pre_authorize(has_any_role = ["ADMIN", "AUDITOR"], mode = "Or")]
/// #[get_mapping(path = "/reports")]
/// async fn reports() -> &'static str {
///     "[]"
/// }
/// ```
#[proc_macro_attribute]
pub fn pre_authorize(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_fn = parse_macro_input!(item as ItemFn);
    impl_macro_pre_authorize(attr, item_fn)
}

/// Declares the rule a handler is guarded with after it ran.
///
/// The rule is a Rust expression over two bindings: `result`, the value the
/// handler returned, and `authentication`, the authentication of the request
/// (`authentication.name` and `authentication.authorities`, and the methods
/// `has_role`, `has_any_role`, `has_authority` and `has_any_authority`).
///
/// The rule is compiled into the handler instead of being interpreted while a
/// request is handled, so the compiler type-checks it together with the handler:
/// a rule reads a field of the result, calls a method of it, indexes a
/// collection, and compares it with anything Rust compares with. A rule that
/// names something the result does not have does not compile, which is the trade
/// this attribute makes on purpose.
///
/// The Spring Security spelling of a rule is understood as well, so
/// `has_role('ADMIN')`, `has_any_role('ADMIN', 'AUDITOR')`,
/// `has_authority('order:read')`, `and`, `or` and `not` mean what they mean
/// there.
///
/// The attribute has to be placed above the `#[get_mapping]`/`#[post_mapping]`/
/// ... of the handler, exactly like [`pre_authorize`], because the routing macros
/// only re-emit the attributes they receive.
///
/// The body of the handler already ran when the rule is evaluated, so a handler
/// that changes the state of the application is guarded with [`pre_authorize`]
/// instead: what a refused request did is not undone.
///
/// # Example
///
/// ```ignore
/// use next_web::macros::{get_mapping, post_authorize};
///
/// #[post_authorize("result == authentication.name")]
/// #[get_mapping(path = "/me")]
/// async fn me() -> String {
///     String::from("max")
/// }
///
/// #[post_authorize("result.owner == authentication.name and result.public")]
/// #[get_mapping(path = "/accounts/{id}")]
/// async fn account() -> Json<Account> {
///     // ...
/// }
/// ```
#[proc_macro_attribute]
pub fn post_authorize(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_fn = parse_macro_input!(item as ItemFn);
    impl_macro_post_authorize(attr, item_fn)
}

/// Implements `Ordered` for the annotated item with the order it is given.
///
/// The trait is the one of `next-web-core`, so the annotated item declares its
/// place among the values the framework orders without an implementation block
/// of its own. The attribute is applied to a type definition (a struct, an enum,
/// or a union) or to an implementation block, and the crate that annotates an
/// item depends on `next-web-core`.
///
/// ```ignore
/// use next_web::macros::order;
///
/// #[order(12)]
/// struct App;
///
/// #[order(-1)]
/// struct Early;
/// ```
///
/// The order is the value the `order` method of `Ordered` returns: a lower value
/// is placed earlier and a higher value later. The value is written as an
/// expression, so a constant such as `i32::MIN` is accepted as well.
#[proc_macro_attribute]
pub fn order(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as syn::Item);

    match generate_order(attr.into(), item) {
        Ok(order) => order.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Process macros for implementing retry logic
///
/// # Description of Attribute Parameters
/// Max attempts: Maximum number of retries, default is 1
/// - ` delay `: The default delay time for each retry is 1000 milliseconds
/// - ` backoff `: Optional backoff policy path
/// - ` retry_for `: a list of error types that need to be retried. Matching these errors will trigger a retry, and the type here should be consistent with the return type of the retry function
/// - ` multiplier `: an optional multiplier expression used to calculate the delay time for each retry
///
/// # Attention
/// The current function should return std:: result:: Result<T, E>type, where T is the normal return value type and E is the error type that needs to be retried
///
/// # Example
/// ```ignore
///
/// #[derive(Debug)]
/// enum TestMatch {
///    A,
///    B(u64),
/// }
///
/// #[Retryable(
///     max_attempts = 3,
///     delay = 100,
///     backoff = test_backoff,
///     retry_for = [TestMatch::A],
///     multiplier = 2
/// )]
/// fn test_retry() -> Result<(), TestMatch> {
///    let timestamp_sec = std::time::SystemTime::now()
///       .duration_since(std::time::UNIX_EPOCH)
///       .unwrap()
///       .as_secs();
///     match timestamp_sec % 2 {
///         0 => Err(TestMatch::B(123)),
///         _ => Err(TestMatch::A),
///     }
/// }
///
/// fn test_backoff(error: &TestMatch) {
///    println! ("function test_retry backoff: {:?}", error);
/// }
/// ```
#[proc_macro_attribute]
pub fn retryable(attr: TokenStream, item: TokenStream) -> TokenStream {
    use crate::web::retry::impl_macro_retry;

    let item_fn = parse_macro_input!(item as ItemFn);
    impl_macro_retry(attr, item_fn)
}

#[proc_macro_attribute]
pub fn event_listener(attr: TokenStream, item: TokenStream) -> TokenStream {
    use crate::web::event::impl_macro_event_listener;

    let item_impl = parse_macro_input!(item as ItemImpl);
    impl_macro_event_listener(attr, item_impl)
}

// ========================= Dependency Injection =========================

/// Declares the annotated item as a singleton provider.
///
/// The instance of a singleton provider is constructed only once and shared by
/// every resolution, so it can be resolved by reference or, when the type
/// implements `Clone`, by value. The annotated type also gets an implementation
/// of `next_web_core::traits::singleton::Singleton` returning the name of the
/// provider.
///
/// The macro can be applied to a `struct`, an `enum`, an `impl` block or a
/// `fn`:
///
/// - on a `struct`, an `enum` or an `impl` block the annotated item is the
///   provided type,
/// - on a `fn` a `struct` named after the function is created and the provided
///   type is the return type of the function. The struct carries no value, it
///   only gives the provider a type of its own.
///
/// # Arguments
///
/// - `name = "..."`: name of the provider. On a `fn` it defaults to the name of
///   the function in camel case, so `fn test_name` provides `testName`; on a
///   `struct` it defaults to the type name with its first character lower
///   cased, e.g. `MyService` becomes `myService`.
/// - `eager_create`: construct the instance while the provider is registered
///   instead of while it is resolved for the first time.
/// - `condition = path` or `condition = |cx| ...`: a function or a closure
///   taking `&dyn ApplicationContext` that decides whether the provider is
///   inserted into the context.
/// - `binds = [path, ...]`: expressions converting the instance into the types
///   it is bound to, e.g. `binds = [Self::into_service]`.
/// - `async`: whether the constructor of a `struct` or an `enum` is async. Not
///   supported on a `fn`, which is async when it is declared as `async fn`.
/// - `auto_register`: whether the provider is submitted to the auto register
///   registry, `true` by default. Generic items have to disable it with
///   `auto_register = false`.
/// - `default`: build the instance with `Default::default()` instead of
///   resolving the fields of the annotated `struct`.
///
/// # Field and argument attributes
///
/// - `#[autowired(name = "...")]`: name of the provider to resolve, which wins
///   over the defaults below. Without it the name of the field or of the
///   argument is used, so `my_service` resolves `myService`; a field or an
///   argument without a name of its own uses the name of its type with its
///   first character lower cased, so `&MyService` resolves `myService`.
/// - `#[autowired(option)]`: the field is an `Option<T>` that is `None` when
///   there is no provider with the given name.
/// - `#[autowired(default)]`: the field uses `Default::default()` when there is
///   no provider with the given name; `#[autowired(default = expr)]` uses the
///   given expression instead.
/// - `#[autowired(vec)]`: the field is a `Vec<T>` collecting every provider of
///   `T`.
/// - `#[autowired(map)]`: the field is a `HashMap<String, T>` or a
///   `BTreeMap<String, T>` naming every provider of `T`. Which map it is comes
///   from the type the field is declared as. Each key is the name the context
///   registered the instance under, so `T` does not have to implement
///   `Singleton` — a type that knows nothing of this framework can be collected
///   by name.
/// - `#[autowired(ref)]`: the field takes a reference to the provider, which is
///   required when a type alias hides the reference of the field type.
/// - A field or an argument declared as a reference, e.g. `&MyService` or
///   `&mut MyService`, is taken from the context as a reference to the instance
///   and does not need `#[autowired(ref)]`; the instance is created in the
///   context first when it does not exist yet. The context lends any number of
///   immutable references, or a single mutable one, so a `&mut` field or
///   argument cannot be combined with another reference.
/// - `#[value(key = "...")]`: the field is *configured* rather than injected —
///   it takes the value of the application property the key names, and no
///   provider is resolved for it. The value is parsed with `FromStr`, so a type
///   that needs a unit, a `Duration` or a byte size, has to be wrapped in a type
///   that parses itself.
///
///   A property that is not configured is not an error: the field takes the value
///   of `default = <expression>` when the attribute names one, and
///   `Default::default()` otherwise. A property that is configured but cannot be
///   parsed is reported as a panic, because that is a mistake rather than an
///   absence. Declare the field as an `Option<T>` when the property may be
///   absent, which is how "not configured" is told apart from a value that is
///   there.
/// - `#[resource(path = ...)]`: path of the crate used by the generated code,
///   `::next_web_context` by default.
///
/// # Example
///
/// ```ignore
/// #[singleton(binds = [Self::into_service])]
/// #[derive(Clone)]
/// struct MyService {
///     #[autowired(name = "myRepository")]
///     repository: Arc<dyn Repository>,
/// }
/// ```
#[proc_macro_attribute]
pub fn singleton(attr: TokenStream, item: TokenStream) -> TokenStream {
    di::generate(attr, item, Scope::Singleton)
}

/// Declares the annotated item as a transient provider.
///
/// The instance of a transient provider is constructed on every resolution, so
/// it is always resolved by value. The macro accepts the same arguments and
/// field attributes as [`singleton`](macro@crate::singleton).
#[proc_macro_attribute]
pub fn transient(attr: TokenStream, item: TokenStream) -> TokenStream {
    di::generate(attr, item, Scope::Transient)
}

/// Declares the annotated item as a single owner provider.
///
/// The instance of a single owner provider is constructed only once and is
/// resolved by reference. The macro accepts the same arguments and field
/// attributes as [`singleton`](macro@crate::singleton).
#[proc_macro_attribute]
pub fn singleowner(attr: TokenStream, item: TokenStream) -> TokenStream {
    di::generate(attr, item, Scope::SingleOwner)
}

// ========================= Resilience =========================

// The five resilience attributes are defined here rather than inside the module
// above, because a procedural macro has to be defined in the crate root. What
// each one means, the two forms it takes, the stacking order and the error
// contract are documented on `resilience` itself and on each macro below.

/// Caps how often the guarded function may be called.
///
/// Takes one permission from a [`RateLimiter`] before running the body. When the
/// permission cannot be had within the limiter's configured `timeout_duration`,
/// the call is rejected without running the body.
///
/// Asynchronous functions only: a limiter asks a permission store, and that store
/// may live in another process, so the permission has to be awaited.
///
/// A rejection is a [`RateLimitExceededError`], converted into the function's
/// error type unless a `fallback` is given.
///
/// ```ignore
/// #[rate_limiter(self.limiters.login)]
/// async fn login(&self, name: &str) -> Result<Token, LoginError> {
///     self.authenticate(name).await
/// }
///
/// // Degrade to a cached answer instead of failing.
/// #[rate_limiter(name = "search", fallback = Ok(Vec::new()))]
/// async fn search(&self, query: &str) -> Result<Vec<Hit>, SearchError> {
///     self.query(query).await
/// }
/// ```
///
/// [`RateLimiter`]: https://docs.rs/next-web-resilience/latest/next_web_resilience/ratelimiter/struct.RateLimiter.html
/// [`RateLimitExceededError`]: https://docs.rs/next-web-resilience/latest/next_web_resilience/error/struct.RateLimitExceededError.html
#[cfg(feature = "resilience")]
#[proc_macro_attribute]
pub fn rate_limiter(attr: TokenStream, item: TokenStream) -> TokenStream {
    apply(Pattern::RateLimiter, attr.into(), item.into()).into()
}

/// Refuses calls to a function whose dependency is known to be failing, and
/// records the outcome of the ones it lets through.
///
/// The body runs only when the [`CircuitBreaker`] permits the call; its success
/// or failure is then recorded, together with how long it took, which is what
/// lets the breaker open, half-open and close again. Because the outcome is
/// recorded, the body is wrapped so that a `?` inside it cannot leave the
/// function before the recording happens.
///
/// Guards synchronous and asynchronous functions.
///
/// Needs the function's error type to implement `std::error::Error + 'static`,
/// because that is what the breaker records; a rejection is a
/// [`CallNotPermittedError`], converted into that error type unless a `fallback`
/// is given.
///
/// ```ignore
/// #[circuit_breaker(self.breakers.payments, fallback = Ok(Receipt::deferred()))]
/// async fn charge(&self, amount: Money) -> Result<Receipt, PaymentError> {
///     self.gateway.charge(amount).await
/// }
/// ```
///
/// [`CircuitBreaker`]: https://docs.rs/next-web-resilience/latest/next_web_resilience/circuitbreaker/struct.CircuitBreaker.html
/// [`CallNotPermittedError`]: https://docs.rs/next-web-resilience/latest/next_web_resilience/error/struct.CallNotPermittedError.html
#[cfg(feature = "resilience")]
#[proc_macro_attribute]
pub fn circuit_breaker(attr: TokenStream, item: TokenStream) -> TokenStream {
    apply(Pattern::CircuitBreaker, attr.into(), item.into()).into()
}

/// Caps how many calls to the guarded function run at the same time.
///
/// Holds a [`Bulkhead`] permit for the whole call and releases it when the call
/// ends, whether it succeeded, failed or returned early. Without a permit the
/// call is rejected: the bulkhead waits for one for at most its configured
/// `max_wait_duration`.
///
/// Asynchronous functions only, because the wait for a permit is a
/// `tokio::sync::Semaphore` acquisition.
///
/// A rejection is a [`BulkheadFullError`], converted into the function's error
/// type unless a `fallback` is given.
///
/// ```ignore
/// #[bulkhead(self.bulkheads.reports)]
/// async fn report(&self, id: ReportId) -> Result<Report, ReportError> {
///     self.render(id).await
/// }
/// ```
///
/// [`Bulkhead`]: https://docs.rs/next-web-resilience/latest/next_web_resilience/bulkhead/struct.Bulkhead.html
/// [`BulkheadFullError`]: https://docs.rs/next-web-resilience/latest/next_web_resilience/error/struct.BulkheadFullError.html
#[cfg(feature = "resilience")]
#[proc_macro_attribute]
pub fn bulkhead(attr: TokenStream, item: TokenStream) -> TokenStream {
    apply(Pattern::Bulkhead, attr.into(), item.into()).into()
}

/// Bounds how long the guarded function may take.
///
/// Runs the body under a [`TimeLimiter`]. When it outlives the limiter's
/// configured `timeout_duration` the future is dropped, so the call really is
/// abandoned rather than merely reported as late, and the limiter's timeout is
/// recorded on its counters and event stream.
///
/// Asynchronous functions only: abandoning a synchronous call is not possible,
/// so there would be nothing to enforce.
///
/// A timeout is a [`TimeoutError`] and a task that ended without a result is an
/// [`AbortedError`]; both are converted into the function's error type unless a
/// `fallback` is given.
///
/// ```ignore
/// #[time_limiter(self.time_limits.search)]
/// async fn search(&self, query: &str) -> Result<Vec<Hit>, SearchError> {
///     self.index.query(query).await
/// }
/// ```
///
/// [`TimeLimiter`]: https://docs.rs/next-web-resilience/latest/next_web_resilience/timelimiter/struct.TimeLimiter.html
/// [`TimeoutError`]: https://docs.rs/next-web-resilience/latest/next_web_resilience/error/struct.TimeoutError.html
/// [`AbortedError`]: https://docs.rs/next-web-resilience/latest/next_web_resilience/error/struct.AbortedError.html
#[cfg(feature = "resilience")]
#[proc_macro_attribute]
pub fn time_limiter(attr: TokenStream, item: TokenStream) -> TokenStream {
    apply(Pattern::TimeLimiter, attr.into(), item.into()).into()
}

/// Repeats the guarded function while it fails.
///
/// Every attempt re-runs the body, so the function is called again with the same
/// arguments: **only guard an operation that is safe to run twice.** A parameter
/// taken by reference is borrowed again; any other parameter is cloned for the
/// next attempt, so it has to implement `Clone` — the macro adds the bound to the
/// signature when the parameter is one of the function's own type parameters.
///
/// The retry's configuration decides how many attempts are made, which failures
/// are worth retrying and how long to wait between them, and its counters and
/// event stream are updated exactly as `Retry::call` would update them.
///
/// Guards synchronous and asynchronous functions. The wait between attempts is
/// `tokio::time::sleep` for the former kind and `std::thread::sleep` for the
/// latter.
///
/// Only a spent attempt budget is a rejection: with a `fallback` the call
/// degrades instead of failing, and without one the failure of the last attempt
/// is returned unchanged, so no `From` implementation is needed. A failure the
/// policy refuses to retry is never degraded — it goes to the caller.
///
/// ```ignore
/// // Three attempts, exponential backoff, from the retry's own configuration.
/// #[retry(self.retries.inventory)]
/// async fn stock(&self, sku: Sku) -> Result<u32, InventoryError> {
///     self.inventory.stock(sku).await
/// }
///
/// // Degrade to a cached count once the budget is spent.
/// #[retry(name = "inventory", fallback = Ok(self.cached_stock(sku)))]
/// async fn stock_fallback(&self, sku: Sku) -> Result<u32, InventoryError> { .. }
/// ```
#[cfg(feature = "resilience")]
#[proc_macro_attribute]
pub fn retry(attr: TokenStream, item: TokenStream) -> TokenStream {
    apply(Pattern::Retry, attr.into(), item.into()).into()
}

/// Marks a function as the handler of a transition.
///
/// The function keeps its name and body, and the macro adds `<name>_handler()`
/// which builds the `OnTransition` value the machine registers. The first
/// parameter has to be `&dyn StateContext<S, E>`, which is where the macro reads
/// the state and the event type from.
///
/// The generated code refers to `::next_web::state_machine` by default, so an
/// application which depends on the umbrella crate with the
/// `enable-state-machine` feature needs nothing else. A project which depends on
/// the state machine crate directly passes `crate = next_web_state_machine`.
///
/// # Example
/// ```ignore
/// #[next_web::on_transition(source = 1, target = 2, event = 100)]
/// fn ship(
///     context: &dyn next_web::state_machine::state_context::StateContext<i32, i32>,
/// ) -> Result<(), BoxError> {
///     println!("shipping");
///
///     Ok(())
/// }
///
/// #[on_transition(crate = next_web_state_machine, target = 3)]
/// fn deliver(context: &dyn StateContext<i32, i32>) -> Result<(), BoxError> {
///     println!("delivering");
///
///     Ok(())
/// }
///
/// machine.add_transition_handler(Arc::new(ship_handler()))?;
/// ```
#[proc_macro_attribute]
pub fn on_transition(attr: TokenStream, item: TokenStream) -> TokenStream {
    crate::state_machine::on_transition::impl_macro_on_transition(attr, item)
}

/// Makes the machine of a configuration part of the application context.
///
/// The attribute sits on `impl StateMachineConfigurer<S, E> for T`, keeps the
/// block and submits the configuration to the inventory of the framework, so the
/// application builds the machine while it configures the context and a component
/// injects `Arc<dyn StateMachine<S, E>>`.
///
/// The configuration has to be a unit struct which derives `Clone`.
///
/// # Arguments
/// * `name = ".."` - the bean name of the machine, `stateMachine` by default
///
/// # Example
/// ```ignore
/// #[derive(Clone)]
/// struct OrderStateMachineConfig;
///
/// #[state_machine(name = "orderStateMachine")]
/// impl StateMachineConfigurer<OrderState, OrderEvent> for OrderStateMachineConfig {
///     fn config_configure(&mut self, config: &mut StateMachineConfigurationBuilder<..>)
///         -> Result<(), BoxError> {
///         config.with_configuration()?.machine_id(String::from("orderStateMachine"));
///         Ok(())
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn state_machine(attr: TokenStream, item: TokenStream) -> TokenStream {
    crate::state_machine::state_machine::impl_macro_state_machine(attr, item)
}
