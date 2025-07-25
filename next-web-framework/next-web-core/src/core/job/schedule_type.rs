


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleType {
    Cron(&'static str),
    // ms
    Repeated(u64),
    /// One shot job.
    ///
    /// This will schedule a job that is only run once after the duration has passed.
    /// ms
    OneShot(u64),
    OneShotAtInstant(std::time::Instant)
}