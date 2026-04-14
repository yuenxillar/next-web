use crate::connection::message::Message;

/// Default message implementation.
pub struct DefaultMessage {
    channel: Vec<u8>,
    body: Vec<u8>,
}

impl DefaultMessage {
    pub fn new(channel: Vec<u8>, body: Vec<u8>) -> Self {
        Self { channel, body }
    }
}

impl Message for DefaultMessage {
    fn body(&self) -> &[u8] {
        &self.body
    }

    fn channel(&self) -> &[u8] {
        &self.channel
    }
}
