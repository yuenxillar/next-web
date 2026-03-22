pub trait ConfigurerBuilder<I> {
    /// Obtain the parent builder to continue the fluent call chain
    fn and(self) -> I;
}
