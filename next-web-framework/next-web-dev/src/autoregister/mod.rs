#[cfg(feature = "mongodb_enabled")]
pub mod mongodb_autoregister;

#[cfg(feature = "job_scheduler")]
pub mod job_scheduler_autoregister;

#[cfg(feature = "redis_enabled")]
pub mod redis_autoregister;

pub mod register_single;

