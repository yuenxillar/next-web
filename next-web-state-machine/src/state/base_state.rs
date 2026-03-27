use crate::{region::Region, StateMachine};

pub struct BaseState<S, E> {
    s: (S, E),
}

impl<S, E> BaseState<S, E> {
    pub fn get_sub_machine(&self) -> &dyn StateMachine<S, E> {
        todo!()
    }

    pub fn get_regions(&mut self) -> &mut [&mut dyn Region<S, E>] {
        todo!()
    }
}
