use crate::messaging::message_headers::MessageHeaders;

pub trait Message<T> {
    fn get_payload(&self) -> Option<&T>;
    fn get_headers(&self) -> &MessageHeaders;
}
