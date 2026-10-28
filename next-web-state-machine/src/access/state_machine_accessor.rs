use crate::access::state_machine_access::StateMachineAccess;

/// Gives access to the parts of a machine.
pub trait StateMachineAccessor<S, E> {
    /// Runs the closure for a region of the machine.
    ///
    /// # Arguments
    /// * `state_machine_access` - the closure which receives the region access
    fn do_with_region(
        &self,
        state_machine_access: Box<dyn Fn(&dyn StateMachineAccess<S, E>)>,
    );
}
