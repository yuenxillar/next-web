/// This trait is used to represent types with a defined order, where the order is represented by integers.
///
/// The implementer must provide an `order` method that returns an `i32`, representing the logical ordering of the value.
/// Lower integers represent earlier positions, while higher integers represent later positions.
///
/// Overall, this will be very useful for requirements that involve sequential execution logic
///
/// # Example
/// ```
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
