pub mod base_health_indicator;
pub mod health_error;
pub mod health_indicator;
pub mod status;
pub mod system_health;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use crate::actuate::health::status::Status;

/// Represents the health information for a component.
///
/// This is the Rust equivalent of Spring Boot's `Health` class.
/// Contains status and optional details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Health {
    status: Status,

    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    details: BTreeMap<String, serde_json::Value>,
}

impl Health {
    /// Creates a new Health instance with the given status and details.
    pub fn new(status: Status, details: BTreeMap<String, serde_json::Value>) -> Self {
        Self { status, details }
    }

    /// Returns the health status.
    pub fn status(&self) -> &Status {
        &self.status
    }

    /// Returns the health details.
    pub fn details(&self) -> &BTreeMap<String, serde_json::Value> {
        &self.details
    }

    /// Returns a new Health instance without details.
    pub fn without_details(self) -> Self {
        if self.details.is_empty() {
            self
        } else {
            Self::with_status(self.status).build()
        }
    }

    /// Creates a health status with a custom Status object.
    pub fn with_status(status: Status) -> HealthBuilder {
        HealthBuilder::with_status(status)
    }
}

impl fmt::Display for Health {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {:?}", self.status, self.details)
    }
}

/// Builder for constructing Health instances.
///
/// This is the Rust equivalent of Spring Boot's `Health.Builder` inner class.
#[derive(Debug, Default, Clone)]
pub struct HealthBuilder {
    status: Status,
    details: BTreeMap<String, serde_json::Value>,
    error: Option<Arc<dyn std::error::Error + Send + Sync>>,
}

impl HealthBuilder {
    /// Creates a new builder with UNKNOWN status.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new builder with the given status.
    pub fn with_status(status: Status) -> Self {
        Self {
            status,
            details: BTreeMap::new(),
            error: None,
        }
    }

    /// Creates a new builder from status and existing details.
    pub fn with_status_and_details(
        status: Status,
        details: BTreeMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            status,
            details,
            error: None,
        }
    }

    /// Adds an error to the health info.
    /// This will automatically add an "error" detail with the error message.
    pub fn with_error<E>(mut self, ex: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        let t_name = std::any::type_name::<E>();
        self.error = Some(Arc::new(ex));

        let error_msg = format!(
            "{}: {}",
            t_name,
            self.error.as_ref().map(AsRef::as_ref).unwrap()
        );

        self.with_detail("error".to_string(), serde_json::Value::String(error_msg))
    }

    /// Adds a detail key-value pair.
    pub fn with_detail(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        let key = key.into();
        let value = serde_json::to_value(value).unwrap_or(serde_json::Value::Null);
        self.details.insert(key, value);
        self
    }

    /// Adds multiple details from a map.
    pub fn with_details<K, V, I>(mut self, details: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Serialize,
    {
        for (key, value) in details {
            let key = key.into();
            let value = serde_json::to_value(value).unwrap_or(serde_json::Value::Null);
            self.details.insert(key, value);
        }
        self
    }

    /// Sets the status to UNKNOWN.
    pub fn unknown(mut self) -> Self {
        self.status = Status::Unknown;
        self
    }

    /// Sets the status to UP.
    pub fn up(mut self) -> Self {
        self.status = Status::Up;
        self
    }

    /// Sets the status to DOWN.
    pub fn down(mut self) -> Self {
        self.status = Status::Down;
        self
    }

    /// Sets the status to DOWN with an error.
    pub fn down_with_error<E>(self, ex: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        self.down().with_error(ex)
    }

    /// Sets the status to OUT_OF_SERVICE.
    pub fn out_of_service(mut self) -> Self {
        self.status = Status::OutOfService;
        self
    }

    /// Sets the status using a status code string.
    pub fn status_code(mut self, code: impl Into<String>) -> Self {
        self.status = Status::with_code(code);
        self
    }

    /// Sets the status explicitly.
    pub fn set_status(mut self, status: Status) -> Self {
        self.status = status;
        self
    }

    /// Gets the current error if present.
    pub fn error(&self) -> Option<&(dyn std::error::Error + Send + Sync)> {
        self.error.as_ref().map(|e| e.as_ref())
    }

    /// Builds the final Health instance.
    pub fn build(self) -> Health {
        Health {
            status: self.status,
            details: self.details,
        }
    }
}
