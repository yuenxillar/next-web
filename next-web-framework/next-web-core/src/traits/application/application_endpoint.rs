/// 定义一个所有应用端点必须实现的 trait。
///
/// 此 trait 用于标记能够提供 Axum 路由器（Router）作为其入口点的类型。
/// 实现此 trait 的类型应通过 `endpoint` 方法返回一个配置好的 `axum::Router`，
/// 该路由器包含了该应用模块所需的所有路由、中间件和处理器。
///
/// # 要求 (Requirements)
///
/// 实现此 trait 的类型必须满足 `Send` 约束，以确保它们可以在多线程环境中安全地跨线程传递。
/// 这对于 Axum 这样的异步运行时通常是必需的。
///
/// # 示例 (Example)
///
/// ```rust
/// use axum::{Router, routing::get};
///
/// struct MyApi;
///
/// impl MyApi {
///     async fn handler() -> &'static str {
///         "Hello, World!"
///     }
/// }
///
/// impl ApplicationEndpoint for MyApi {
///     fn endpoint() -> Router {
///         Router::new().route("/hello", get(Self::handler))
///     }
/// }
/// ```
///
/// # English
///
/// Defines a trait that all application endpoints must implement.
///
/// This trait is used to mark types that can provide an `axum::Router` as their entry point.
/// Types implementing this trait should return a configured `axum::Router` via the `endpoint` method,
/// which contains all the routes, middleware, and handlers required for that application module.
///
/// # Requirements
///
/// Types implementing this trait must satisfy the `Send` bound, ensuring they can be safely sent across threads.
/// This is typically required for async runtimes like Axum.
///
/// # Example
///
/// ```rust
/// use axum::{Router, routing::get};
///
/// struct MyApi;
///
/// impl MyApi {
///     async fn handler() -> &'static str {
///         "Hello, World!"
///     }
/// }
///
/// impl ApplicationEndpoint for MyApi {
///     fn endpoint() -> Router {
///         Router::new().route("/hello", get(Self::handler))
///     }
/// }
/// ```
pub trait ApplicationEndpoint: Send {
    /// 创建并返回此应用模块的根路由器（Router）。
    ///
    /// # 返回值 (Returns)
    ///
    /// 返回一个 `axum::Router` 实例，该实例已配置好此应用模块所需的所有路由。
    ///
    /// # English
    ///
    /// Creates and returns the root router for this application module.
    ///
    /// # Returns
    ///
    /// Returns an `axum::Router` instance configured with all the routes required by this module.
    fn endpoint() -> axum::Router;
}
