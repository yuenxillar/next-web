use crate::{traits::group::Group, util::singleton::SingletonUtil};

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
        SingletonUtil::name::<Self>()
    }
}

/// Implementation of the Group trait for singleton objects
impl<T: ?Sized + Singleton> Group for T {}
