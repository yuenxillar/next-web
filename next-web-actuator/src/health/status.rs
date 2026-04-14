use std::{default, fmt};

use serde::{Deserialize, Serialize};

/// Represents the health status of a component.
///
/// This is the Rust equivalent of Spring Boot's `Status` class.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Status {
    #[default]
    Unknown,
    Up,
    Down,
    OutOfService,
    Custom(String, Option<String>),
}

impl Status {
    /// Creates a custom status with the given code.
    pub fn with_code(code: impl Into<String>) -> Self {
        let code = code.into();
        Self::Custom(code, Default::default())
    }

    /// Creates a custom status with code and description.
    pub fn with_code_and_description(
        code: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        let code = code.into();
        let description = Some(description.into());
        Self::Custom(code, description)
    }

    pub fn code(&self) -> &str {
        match self {
            Self::Unknown => "Unknown",
            Self::Up => "Up",
            Self::Down => "Down",
            Self::OutOfService => "OutOfService",
            Self::Custom(code, _) => code,
        }
    }

    pub fn description(&self) -> Option<&str> {
        match self {
            Self::Custom(_, description) => description.as_deref(),
            _ => None,
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}
