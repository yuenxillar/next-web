use crate::{messaging::message_headers::MessageHeaders, traits::message::Message};

#[derive(Clone)]
pub struct GenericMessage<T> {
    payload: Option<T>,
    headers: MessageHeaders,
}

impl<T> GenericMessage<T> {
    pub fn new(payload: Option<T>, headers: MessageHeaders) -> Self {
        GenericMessage { payload, headers }
    }
}

impl<T> Message<T> for GenericMessage<T>
where
    T: Send + Sync,
    T: Clone,
{
    fn get_payload(&self) -> Option<&T> {
        self.payload.as_ref()
    }

    fn get_own_payload(&mut self) -> Option<T> {
        self.payload.take()
    }

    fn get_headers(&self) -> &MessageHeaders {
        &self.headers
    }
}
