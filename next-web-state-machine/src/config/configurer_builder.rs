/// A configurer which hands the fluent chain back to its parent.
///
/// # Type Parameters
/// * `I` - the parent configurer the chain continues with
pub trait ConfigurerBuilder<I> {
    /// Obtain the parent builder to continue the fluent call chain
    fn and(&mut self) -> I;
}
