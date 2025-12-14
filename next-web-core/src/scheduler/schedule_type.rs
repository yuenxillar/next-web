#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduleType {
    Cron(WithArgs),
    // second
    FixedRate(WithArgs),
    /// One shot job.
    ///
    /// This will schedule a job that is only run once after the duration has passed.
    /// second
    OneShot(WithArgs),
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct WithArgs {
    pub cron: Option<&'static str>,
    pub fixed_rate: Option<u64>,
    pub initial_delay: Option<u64>,
    pub timezone: Option<&'static str>,

    pub time_unit: Option<&'static str>,
}