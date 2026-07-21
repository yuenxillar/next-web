/// This interface represents the abstract notion of a Principal,
/// which can be used to represent any entity, such as an individual, a corporation, and a login id.
pub trait Principal {
    /// Returns the name of this Principal.
    fn name(&self) -> &str;
}
