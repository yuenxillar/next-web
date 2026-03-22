pub struct StateMachineConfig<S, E> {
    // Define fields here
    var: std::marker::PhantomData<(S, E)>,
}

impl<S, E> Default for StateMachineConfig<S, E> {
    fn default() -> Self {
        Self {
            var: std::marker::PhantomData,
        }
    }
}
