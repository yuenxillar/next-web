use next_web_core::error::BoxError;

/// Interface for building an object.
///
/// # Type Parameters
/// * `O` - The type of the object being built
pub trait Builder<O> {
    /// Builds the object and returns it.
    ///
    /// # Returns
    /// * `Result<O, Box<dyn Error>>` - The built object or an error if building failed
    fn build(&mut self) -> Result<O, BoxError>;
}
