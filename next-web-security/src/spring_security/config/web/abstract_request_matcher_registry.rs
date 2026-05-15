#[derive(Clone)]
pub struct AbstractRequestMatcherRegistry<C> {
    pub(crate) _marker: std::marker::PhantomData<C>,
}

impl<C> AbstractRequestMatcherRegistry<C> {}

impl<C> Default for AbstractRequestMatcherRegistry<C> {
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}
