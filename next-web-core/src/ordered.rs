/// This trait is used to represent types with a defined order, where the order is represented by integers.
///
/// The implementer must provide an `order` method that returns an `i32`, representing the logical ordering of the value.
/// Lower integers represent earlier positions, while higher integers represent later positions.
///
/// Overall, this will be very useful for requirements that involve sequential execution logic
///
/// # Example
/// ```
/// struct TestOrder;
///
/// impl Ordered for TestOrder {
///     fn order(&self) -> i32 {
///         100
///     }
/// }
/// ```
pub trait Ordered {
    /// Returns the order of the value, as an `i32`.
    ///
    /// Lower integers represent earlier positions, while higher integers represent later positions.
    fn order(&self) -> i32 {
        100
    }
}
