use uuid::Uuid;

use crate::error::IdGeneratorError;
use crate::generator::IdGenerator;

/// Generates UUID v4 strings.
#[derive(Debug, Default, Clone, Copy)]
pub struct UuidGenerator;

impl UuidGenerator {
    /// Returns a canonical UUID string with hyphens.
    pub fn next_hyphenated(&self) -> String {
        Uuid::new_v4().to_string()
    }

    /// Returns a compact UUID string without hyphens.
    pub fn next_compact(&self) -> String {
        Uuid::new_v4().simple().to_string()
    }

    /// Returns the raw UUID value.
    pub fn next_uuid(&self) -> Uuid {
        Uuid::new_v4()
    }
}

impl IdGenerator<String> for UuidGenerator {
    fn next_id(&self) -> Result<String, IdGeneratorError> {
        Ok(self.next_hyphenated())
    }
}
