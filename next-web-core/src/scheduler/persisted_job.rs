use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::scheduler::schedule_type::ScheduleType;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersistedScheduledJob {
    pub id: String,
    pub task_key: String,
    pub schedule: ScheduleType,
    pub enabled: bool,
    pub payload: Option<Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub version: u64,
}

impl PersistedScheduledJob {
    pub fn new(id: impl Into<String>, task_key: impl Into<String>, schedule: ScheduleType) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            task_key: task_key.into(),
            schedule,
            enabled: true,
            payload: None,
            created_at: now,
            updated_at: now,
            last_run_at: None,
            version: 1,
        }
    }

    pub fn with_payload(mut self, payload: Value) -> Self {
        self.payload = Some(payload);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
        self.version = self.version.saturating_add(1);
    }

    pub fn mark_ran_now(&mut self) {
        self.last_run_at = Some(Utc::now());
        self.touch();
    }
}
