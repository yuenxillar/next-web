pub trait Message {
    /// Returns the body (or the payload) of the message.
    fn body(&self) -> &[u8];

    /// Returns the channel associated with the message.
    fn channel(&self) -> &[u8];
}
