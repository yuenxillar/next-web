/// Enumerations for possible state do action policies.
///
/// A state action runs after the entry actions of its state. The runtime awaits
/// it inline, so the policies differ in whether the action is bounded by a
/// timeout. Exiting the state cannot interrupt an action which the runtime is
/// still awaiting, which the Java implementation allows because it runs the
/// action in its own task.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum StateDoActionPolicy {
    /// The state action runs inline and is not bounded by a timeout.
    ///
    /// This is the default policy.
    ImmediateCancel,

    /// The state action is bounded by the timeout of the state, or by the
    /// `timeout` message header, and is skipped when the timeout expires.
    TimeoutCancel,
}
