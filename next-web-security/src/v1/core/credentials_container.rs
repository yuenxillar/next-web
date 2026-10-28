/// Indicates that the implementing object contains sensitive data, which can be erased using the
/// eraseCredentials method. Implementations are expected to invoke the method on any
/// internal objects which may also implement this interface.
///
/// For internal framework use only. Users who are writing their own AuthenticationProvider implementations should
/// create and return an appropriate Authentication object there, minus any sensitive data,
/// rather than using this interface.
pub trait CredentialsContainer {
    fn erase_credentials(&self);
}
