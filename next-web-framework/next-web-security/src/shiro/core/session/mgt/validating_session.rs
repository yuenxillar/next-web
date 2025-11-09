use next_web_core::error::BoxError;

pub trait ValidatingSession: Send + Sync {
    fn is_valid(&self) -> bool;

    fn validate(&self) -> Result<(), BoxError>;
}
