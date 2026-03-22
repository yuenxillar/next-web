pub trait LocalTransitionConfigurer<S, E> {
    fn target(&mut self, target: S) -> &mut dyn LocalTransitionConfigurer<S, E>;
}
