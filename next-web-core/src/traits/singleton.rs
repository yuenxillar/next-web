/// 单例
///
/// 该 trait 定义了单例对象的基本行为，要求实现类型是线程安全的（Send + Sync）
///
/// 目前只有 单例名称 方法
///
/// # 类型约束
/// - `Send`: 允许在线程间安全传递
/// - `Sync`: 允许通过共享引用在线程间安全访问
///
/// # 示例
/// ```
/// use std::sync::Arc;
/// use next_web_core::traits::singleton::Singleton;
///
/// struct MyService;
///
/// impl Singleton for MyService {
///     // 使用默认的 singleton_name 实现
/// }
///
/// ```
/// Single
///
/// This trait defines the basic behavior of singleton objects and requires the implementation type to be thread safe (Send + Sync)
///
/// Currently, there is only a single instance name method available
///
/// # Type Constraint
/// - `Send`: Allow secure transmission between threads
/// - `Sync`: Allow secure access between threads through shared references
///
/// # Example
/// ```
/// use std::sync::Arc;
/// use next_web_core::traits::singleton::Singleton;
///
/// struct MyService;
///
/// impl Singleton for MyService {
///    // Implement using the default singleton_name
/// }
///
/// ```
pub trait Singleton
where
    Self: Send + Sync,
{
    fn singleton_name(&self) -> String {
        let type_name = std::any::type_name::<Self>();
        let name = type_name.rsplit("::").next().unwrap_or_default();

        let mut characters = name.chars();
        match characters.next() {
            Some(first) => {
                let mut default_name = String::with_capacity(name.len());
                default_name.extend(first.to_lowercase());
                default_name.push_str(characters.as_str());
                default_name
            }
            None => name.to_string(),
        }
    }
}
