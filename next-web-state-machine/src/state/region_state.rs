use std::marker::PhantomData;

pub struct RegionState<S, E> {
    var: PhantomData<(S, E)>,
}
