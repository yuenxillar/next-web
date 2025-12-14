use next_web_core::error::BoxError;

pub trait PasswordEncoder
where
    Self: Send + Sync,
{
    fn encode(&self, raw_password: &str) -> Result<String, BoxError>;

    fn matches(&self, raw_password: &str, encoded_password: &str) -> bool;

    #[allow(unused_variables)]
    fn upgrade_encoding(&self, encoded_password: &str) -> bool {
        false
    }
}
