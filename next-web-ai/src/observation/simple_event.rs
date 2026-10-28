use core::time;

use crate::observation::observation::Event;

pub struct SimpleEvent {
    name: String,
    contextual_name: String,
    wall_time: u64,
}

impl SimpleEvent {
    pub fn new<T>(name: T, contextual_name: T) -> Self
    where
        T: Into<String>,
    {
        Self {
            name: name.into(),
            contextual_name: contextual_name.into(),
            wall_time: current_time_seconds(),
        }
    }
}

#[inline]
pub fn current_time_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

impl Event for SimpleEvent {
    fn name(&self) -> &str {
        self.name.as_str()
    }
}
