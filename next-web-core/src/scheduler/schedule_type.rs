use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WithArgs {
    pub cron: Option<String>,
    pub fixed_rate: Option<u64>,
    pub initial_delay: Option<u64>,
    pub timezone: Option<String>,

    pub time_unit: Option<String>,
}
