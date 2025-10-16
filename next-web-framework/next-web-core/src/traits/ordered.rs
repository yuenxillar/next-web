/// 定义具有顺序的类型，顺序由整数表示
///
/// 实现者必须提供一个 `order` 方法，该方法返回一个 `i32`，表示值的逻辑顺序
/// 较低的整数表示较早的位置，较高的整数表示较晚的位置。
///
/// 总体而言， 对于有先后顺序执行逻辑的需求，这将很有用
///
/// # 示例
/// ```
/// # use your_crate::Ordered;
/// #[derive(Debug)]
/// struct Order {}
///
/// impl Ordered for Order {
///     fn order(&self) -> i32 {
///         100
///     }
/// }
/// ```
///
/// This trait is used to represent types with a defined order, where the order is represented by integers.
///
/// The implementer must provide an `order` method that returns an `i32`, representing the logical ordering of the value.
/// Lower integers represent earlier positions, while higher integers represent later positions.
///
/// Overall, this will be very useful for requirements that involve sequential execution logic
///
/// # Example
/// ```
/// # use your_crate::Ordered;
/// #[derive(Debug)]
/// struct Order {}
///
/// impl Ordered for Order {
///     fn order(&self) -> i32 {
///         100
///     }
/// }
/// ```
pub trait Ordered {
    fn order(&self) -> i32;
}
