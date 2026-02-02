use crate::{messaging::message_headers::MessageHeaders, traits::message::Message};

pub struct GenericMessage<T> {
    payload: Option<T>,
    headers: MessageHeaders,
}

impl<T> GenericMessage<T> {
    pub fn new(payload: Option<T>, headers: MessageHeaders) -> Self {
        GenericMessage { payload, headers }
    }
}

impl<T> Message<T> for GenericMessage<T> {
    fn get_payload(&self) -> Option<&T> {
        self.payload.as_ref()
    }

    fn get_headers(&self) -> &MessageHeaders {
        &self.headers
    }
}
