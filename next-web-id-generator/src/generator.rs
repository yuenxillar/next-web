use crate::error::IdGeneratorError;

/// Common trait for generators that produce unique IDs.
pub trait IdGenerator<T> {
    /// Generates the next unique ID.
    fn next_id(&self) -> Result<T, IdGeneratorError>;
}
