use crate::state_machine::access::state_machine_access::StateMachineAccess;

pub trait StateMachineAccessor<S, E> {
    fn do_with_region(
        &self,
        state_machine_access: Box<dyn Fn(&dyn StateMachineAccess<S, E>) -> ()>,
    );
}
