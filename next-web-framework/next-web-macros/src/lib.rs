extern crate proc_macro;

use crate::data::builder::impl_macro_builder;
use crate::data::constructor::impl_macro_required_args_constructor;
use crate::data::field_name::impl_macro_field_name;
use crate::data::get_set::impl_macro_get_set;
use crate::web::idempotency::impl_macro_idempotency;
use crate::web::pre_authorize::impl_macro_pre_authorize;
use crate::web::properties::impl_macro_properties;
use crate::web::retry::impl_macro_retry;
use crate::web::scheduled::impl_macro_scheduled;

use data::desensitized::impl_macro_desensitized;
use proc_macro::TokenStream;
use syn::parse_macro_input;
use syn::DeriveInput;
use syn::ItemFn;
use syn::ItemStruct;

mod common;
mod data;
mod database;
mod singleton;
mod util;
mod web;

/// 为结构体自动生成 getter 和 setter 方法。
///
/// 该宏会为结构体的每个字段生成对应的 `get_{field}` 和 `set_{field}` 方法，
/// 除非字段上使用了 `#[get_set(...)]` 属性进行跳过控制。
///
/// # 字段属性控制
///
/// - `#[get_set(skip)]`：跳过该字段的 getter 和 setter 生成。
/// - `#[get_set(skip_get)]`：仅跳过 getter 方法的生成。
/// - `#[get_set(skip_set)]`：仅跳过 setter 方法的生成。
///
/// # 示例
///
/// ```rust
/// #[derive(GetSet)]
/// struct MyStruct {
///     #[get_set(skip)]
///     field1: i32,        // 不生成任何方法
///     field2: i32,        // 生成 get_field2() 和 set_field2()
///     #[get_set(skip_get)]
///     field3: i32,        // 仅生成 set_field3()
///     #[get_set(skip_set)]
///     field4: i32,        // 仅生成 get_field4()
/// }
/// ```
///
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

/// 为结构体的每个字段生成对应获取的字段名的方法 （字符串字面量）
///
/// 方法名为 `field_{field_name}`, 返回类型为 `&'static str`（如 `field_name`）, 值为字段的原始名称（如 `"name"`）.
///
/// # 示例
///
/// ```rust
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
///
/// Method for generating corresponding field names for each field of a structure (string literal)
///
/// The method name is' field_ {field_name} ', the return type is' static str' (such as' field_name '), and the value is the original name of the field (such as' name')
///
/// # Example
///
/// ```rust
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

/// 为结构体实现构建器（Builder）模式
///
/// 该宏会生成一个同名的 `Builder` 结构体，并为每个字段提供链式调用的 setter 方法
/// 最终通过 `build()` 方法构造原始结构体实例
///
/// ## 注意
/// 1. 默认情况下，即使字段为 Option , 在build方法之前使用者也需要对这个字段进行赋值, 除非使用了 default 辅助参数
///
/// 支持通过 `#[builder(...)]` 属性对字段行为进行定制<br>
/// 辅助参数:<br>
///  `into`: builer 所生成的方法的入参将会是一个实现了 `Into<field>` 的类型<br>
///  `default`:<br>   1. 在 build 方法中, 如果 builder 结构体中这个字段没有用户的输入, 将会使用 Default::default() 来赋值给原始实例<br>
///                   2. 如 #\[builder(default = "default_name")\], 将会使用该函数返回值进行赋值
///
/// # 示例
///
/// ```rust
/// #[derive(Builder)]
/// struct User {
///     #[builder(into)]
///     name: String,
///     age: u32,
/// }
///
/// // 生成 UserBuilder 结构体
///  sturct UserBuilder {
///     name: Option<String>,
///     age: Option<u32>,
/// }
///
/// let user = User::builder()
///     .name("Alice")
///     .age(30)
///     .build()
///     .unwrap();
/// ```
///
/// Implement the Builder pattern for structures
///
/// This macro will generate a 'Builder' struct with the same name and provide a chain call setter method for each field
/// Finally, the 'build()' method is used to construct the original structure instance
///
/// ## Attention
///     1. By default, even if the field is Option, the user still needs to assign a value to this field before the build method, unless the default auxiliary parameter is used
///
/// Support customizing field behavior through the '# [builder (...)]' attribute<br>
/// Auxiliary parameters:<br>
/// Into: The input parameter of the method generated by the boiler will be a type that implements Into<field><br>
/// ` default `:<br>1. In the build method, if there is no user input for this field in the builder structure, Default:: default() will be used to assign it to the original instance<br>
///                   2. If #\[builder (default="default_name")\], the return value of this function will be used for assignment
///
/// # Example
///
/// ```rust
/// #[derive(Builder)]
/// struct User {
///     #[builder(into)]
///     name: String,
///     age: u32,
/// }
///
/// // Generate UserBuilder structure
///  sturct UserBuilder {
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

/// 为结构体实现有参构造函数
///
/// 该宏会为结构体生成一个默认构造函数，并为每个字段提供对应的参数，
/// 最终通过参数构造原始结构体实例
///
/// ## 注意
///
/// 1. 如果字段为 Option 类型, 那么在参数列表中将不会有该字段的入参, 除非使用了 required 辅助参数
///
/// 辅助参数:
///
/// `required`:  那么即使字段为 Option 也将会出现在方法入参中
/// `default`:   无此字段入参, 使用 Default::default() 进行赋值
/// `into`:      入参将会以 impl Into<field> 的类型进行转换
///
/// # 示例
///
/// ```rust
/// #[derive(RequiredArgsConstructor)]
/// struct Person {
///     #[constructor(into)]
///     name: String,
///     age: u32,
/// }
///
/// let person = Person::from_args("Alice", 30);
/// ```
///
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
/// ```rust
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

#[proc_macro_derive(Desensitized, attributes(de))]
pub fn desensitized(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    impl_macro_desensitized(&input)
}

// =============================== Web ===============================
//
#[doc = ""]
#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn Properties(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ItemStruct);
    impl_macro_properties(attr, item)
}

#[doc = ""]
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn RequestMapping(args: TokenStream, input: TokenStream) -> TokenStream {
    crate::web::routing::with_method(None, args, input)
}

macro_rules! method_macro {
    ($method:ident, $variant:ident) => {
        #[doc = ""]
        #[proc_macro_attribute]
        #[allow(non_snake_case)]
        pub fn $method(args: TokenStream, input: TokenStream) -> TokenStream {
            crate::web::routing::with_method(
                Some(crate::web::routing::Method::$variant),
                args,
                input,
            )
        }
    };
}

method_macro!(GetMapping, Get);
method_macro!(PostMapping, Post);
method_macro!(PutMapping, Put);
method_macro!(DeleteMapping, Delete);
method_macro!(PatchMapping, Patch);
method_macro!(AnyMapping, Any);

#[doc = "A procedural macro attribute for defining scheduled tasks.\n\n\
         This attribute can be applied to a function to register it as a scheduled job\n\
         with configurable timing behavior. It supports three scheduling modes:\n\n\
         - **Cron-based scheduling**: via the `cron` parameter (e.g., `\"0 0 2 * * *\"`).\n\
         - **Fixed-rate execution**: via the `fixed_rate` parameter (executes repeatedly at fixed intervals).\n\
         - **One-shot execution**: when `one_shot = true`, the task runs once after an optional `initial_delay`.\n\n\
         # Parameters\n\n\
         - `cron`: A cron expression in 6-field format (seconds, minutes, hours, day-of-month, month, day-of-week). \n\
           Mutually exclusive with `fixed_rate`.\n\
         - `fixed_rate`: Interval between executions (as a positive integer literal). \n\
           Mutually exclusive with `cron`.\n\
         - `initial_delay`: Delay before the first execution (in units specified by `time_unit`).\n\
         - `timezone`: IANA time zone ID (e.g., `\"Asia/Shanghai\"`, `\"UTC\"`). \n\
           If empty or omitted, the scheduler's default time zone is used.\n\
         - `time_unit`: Time unit for `fixed_rate` and `initial_delay` (e.g., `\"ms\"`, `\"s\"`, `\"m\"`). \n\
           Interpretation depends on the underlying scheduler.\n\
         - `one_shot`: If `true`, the task runs exactly once (typically after `initial_delay`). \n\
           In this mode, `cron` and `fixed_rate` are ignored.\n\n\
         # Examples\n\n\
         ```rust\n\
         #[Scheduled(cron = \"0 0 3 * * *\", timezone = \"UTC\")]\n\
         fn daily_cleanup() { /* ... */ }\n\n\
         #[scheduled(fixed_rate = 30, time_unit = \"s\", initial_delay = 5)]\n\
         fn heartbeat() { /* ... */ }\n\n\
         #[Scheduled(one_shot = true, initial_delay = 10, time_unit = \"s\")]\n\
         fn delayed_init() { /* ... */ }\n\
         ```"]
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn Scheduled(args: TokenStream, input: TokenStream) -> TokenStream {
    crate::web::scheduled::impl_macro_scheduled(args, input)
}

/// 实现幂等性属性的过程宏
///
/// # 属性参数说明
/// - `name`:   可选字符串字面量，指定 IdempotencyStore 的单例名称, 可以指定名称使用自己实现的 Store, 默认为`memoryIdempotencyStore`
/// - `key`:    可选字符串字面量，用于获取请求头的键值，默认值为 `Idempotency-key`
/// - `cache_key_prefix`: 可选字符串字面量，应用再缓存键的前缀
/// - `ttl`: 可选整数字面量，缓存存活时间(Time To Live), 默认为秒
///
/// # 注意
/// 当前宏 只支持 Mehod 为 `POST`
/// 你应该使用 `PostMapping` 或者 `RequestMapping(method = "POST")`
///
/// # 示例
/// ```rust
/// #[Idempotency(name = "myIdempotencyStore", key = "Idempotency-key", cache_key_prefix = "test", ttl = 6)]
/// #[PostMapping(path = "/createOrder")]
/// async fn create_order(order: String) -> impl IntoResponse {
///
///     "Ok"
/// }
///
/// ```
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
/// ```rust
/// #[Idempotency(name = "myIdempotencyStore", key = "Idempotency-key", cache_key_prefix = "test", ttl = 6)]
/// #[PostMapping(path = "/createOrder")]
/// async fn create_order(order: String) -> impl IntoResponse {
///
///     "Ok"
/// }
///
/// ```
#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn Idempotency(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_fn = parse_macro_input!(item as ItemFn);
    impl_macro_idempotency(attr, item_fn)
}

#[doc = ""]
#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn PreAuthorize(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_fn = parse_macro_input!(item as ItemFn);
    impl_macro_pre_authorize(attr, item_fn)
}

/// 实现可重试逻辑的过程宏
///
/// # 属性参数说明
/// - `max_attempts`:   最大重试次数，默认为 1
/// - `delay`:          每次重试的延迟时间，默认为 1000 毫秒
/// - `backoff`:        可选退避策略路径
/// - `retry_for`:      需要重试的错误类型列表，匹配这些错误时会触发重试, 这里的类型应该和重试函数的返回类型一致
/// - `multiplier`:     可选乘数表达式，用于计算每次重试的延迟时间
///
/// # 注意
/// 当前函数应返回 std::result::Result<T, E> 类型，其中 T 为正常返回值类型，E 为需要重试的错误类型
///
/// # 示例
///
/// ```rust
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
///    println!("function test_retry backoff: {:?}", error);
/// }
/// ```
///
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
/// ```rust
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
#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn Retryable(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_fn = parse_macro_input!(item as ItemFn);
    impl_macro_retry(attr, item_fn)
}
