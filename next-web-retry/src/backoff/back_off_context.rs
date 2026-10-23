//! State kept by a back-off policy for the duration of one retry block.

use std::any::Any;

/// State kept by a [`BackOffPolicy`](super::back_off_policy::BackOffPolicy) for the duration of
/// one retry block.
///
/// The template creates the context once through
/// [`BackOffPolicy::start`](super::back_off_policy::BackOffPolicy::start), hands it back on
/// every wait, and stores it in the retry context so a stateful retry resumes with the same
/// schedule. It is a marker trait: the policy that created the value is the only one that
/// interprets it, and it downcasts to its own type.
pub trait BackOffContext
where
    Self: Send + Sync,
    Self: Any,
{
}

impl<T> BackOffContext for T
where
    T: Send + Sync,
    T: Any,
{
}

#[cfg(test)]
mod tests {
    use super::BackOffContext;

    fn assert_context<T: BackOffContext>() {}

    #[test]
    fn any_owned_thread_safe_value_can_be_a_context() {
        assert_context::<u64>();
        assert_context::<String>();
    }
}
