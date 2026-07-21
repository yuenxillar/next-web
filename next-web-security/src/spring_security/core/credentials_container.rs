pub trait CredentialsContainer: Send + Sync {
    fn erase_credentials(&self);
}
