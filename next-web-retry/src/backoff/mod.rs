//! Back-off policies.
//!
//! A back-off policy decides how long to wait before the next attempt. The package offers
//! the four schedules the original implementation defines — none, fixed, uniform random and
//! exponential (with or without jitter) — plus the sleeping abstraction they are built on.

pub mod back_off_context;
pub mod back_off_policy;
pub mod back_off_policy_builder;
pub mod exponential_back_off_policy;
pub mod exponential_random_back_off_policy;
pub mod fixed_back_off_policy;
pub mod no_back_off_policy;
pub mod sleeper;
pub mod sleeping_back_off_policy;
pub mod stateless_back_off_policy;
pub mod uniform_random_back_off_policy;

pub use back_off_context::BackOffContext;
pub use back_off_policy::BackOffPolicy;
pub use back_off_policy_builder::{BackOffPolicyBuilder, BuiltBackOffPolicy};
pub use exponential_back_off_policy::{
    DEFAULT_INITIAL_INTERVAL, DEFAULT_MAX_INTERVAL, DEFAULT_MULTIPLIER, ExponentialBackOffContext,
    ExponentialBackOffPolicy,
};
pub use exponential_random_back_off_policy::ExponentialRandomBackOffPolicy;
pub use fixed_back_off_policy::FixedBackOffPolicy;
pub use no_back_off_policy::NoBackOffPolicy;
#[allow(deprecated)]
pub use sleeper::{
    ImmediateSleeper, ObjectWaitSleeper, RecordingSleeper, Sleeper, ThreadWaitSleeper,
};
pub use sleeping_back_off_policy::SleepingBackOffPolicy;
pub use stateless_back_off_policy::StatelessBackOffPolicy;
pub use uniform_random_back_off_policy::UniformRandomBackOffPolicy;
