use crate::core::session::SessionError;

pub trait ValidatingSession: Send + Sync {
    fn is_valid(&self) -> bool;

    fn validate(&self) -> Result<(), SessionError>;
}
