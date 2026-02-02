/// Trait for desensitizing data.
pub trait Desensitized: Sync + Send {
    /// Desensitizes the data.
    fn desensitize(&mut self);
}
