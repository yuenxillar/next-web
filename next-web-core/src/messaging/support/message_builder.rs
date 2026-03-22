use std::collections::HashMap;
use std::fmt::Debug;

use crate::anys::any_value::AnyValue;
use crate::messaging::generic_message::GenericMessage;
use crate::messaging::message_channel::MessageChannel;
use crate::messaging::message_headers::MessageHeaders;
use crate::messaging::support::message_header_accessor::MessageHeaderAccessor;
use crate::traits::any_clone::AnyClone;
use crate::traits::message::Message;

/// A builder for creating messages.
pub struct MessageBuilder<T> {
    payload: Option<T>,
    provided_message: Option<Box<dyn Message<T>>>,
    header_accessor: MessageHeaderAccessor,
}

impl<T> MessageBuilder<T>
where
    T: Clone,
{
    /// Private constructor from a provided message.
    fn from_provided_message(mut provided_message: Box<dyn Message<T>>) -> Self {
        let payload = provided_message.get_own_payload();
        let header_accessor = MessageHeaderAccessor::from_message(provided_message.clone());

        MessageBuilder {
            payload,
            provided_message: Some(provided_message),
            header_accessor,
        }
    }

    /// Private constructor with payload and accessor.
    fn with_payload_and_accessor(payload: T, header_accessor: MessageHeaderAccessor) -> Self {
        MessageBuilder {
            payload: Some(payload),
            provided_message: None,
            header_accessor,
        }
    }

    /// Set the headers using a MessageHeaderAccessor.
    pub fn set_headers(mut self, accessor: MessageHeaderAccessor) -> Self {
        self.header_accessor = accessor;
        self
    }

    /// Set a header value.
    pub fn set_header(mut self, header_name: String, header_value: AnyValue) -> Self {
        self.header_accessor.set_header(header_name, header_value);
        self
    }

    /// Set a header value if it doesn't already exist.
    pub fn set_header_if_absent(mut self, header_name: String, header_value: AnyValue) -> Self {
        self.header_accessor
            .set_header_if_absent(header_name, header_value);
        self
    }

    /// Remove headers matching the given patterns.
    pub fn remove_headers(mut self, header_patterns: Vec<String>) -> Self {
        self.header_accessor.remove_headers(header_patterns);
        self
    }

    /// Remove a specific header.
    pub fn remove_header(mut self, header_name: impl AsRef<str>) -> Self {
        self.header_accessor.remove_header(header_name.as_ref());
        self
    }

    /// Copy headers from a map.
    pub fn copy_headers(mut self, headers: HashMap<String, AnyValue>) -> Self {
        self.header_accessor.copy_headers(headers);
        self
    }

    /// Copy headers from a map if they don't already exist.
    pub fn copy_headers_if_absent(mut self, headers: HashMap<String, AnyValue>) -> Self {
        self.header_accessor.copy_headers_if_absent(headers);
        self
    }

    /// Set the reply channel.
    pub fn set_reply_channel<T1>(mut self, reply_channel: T1) -> Self
    where
        T1: MessageChannel<T> + AnyClone,
        T: Send + Sync,
    {
        self.header_accessor.set_reply_channel(reply_channel);
        self
    }

    /// Set the reply channel name.
    pub fn set_reply_channel_name(mut self, reply_channel_name: impl Into<String>) -> Self {
        self.header_accessor
            .set_reply_channel_name(reply_channel_name.into());
        self
    }

    /// Set the error channel.
    pub fn set_error_channel<T1>(mut self, error_channel: T1) -> Self
    where
        T1: MessageChannel<T> + AnyClone,
        T: Send + Sync,
    {
        self.header_accessor.set_error_channel(error_channel);
        self
    }

    /// Set the error channel name.
    pub fn set_error_channel_name(mut self, error_channel_name: impl Into<String>) -> Self {
        self.header_accessor
            .set_error_channel_name(error_channel_name.into());
        self
    }

    /// Build the message.
    pub fn build(self) -> Box<dyn Message<T>>
    where
        T: Send + Sync,
        T: Clone,
        T: 'static,
    {
        // If we have a provided message and the header accessor hasn't been modified, return the original
        if self.provided_message.is_some() && !self.header_accessor.is_modified() {
            return self.provided_message.unwrap();
        }

        let headers_to_use = self.header_accessor.to_message_headers();

        // Handle error messages
        // todo!()

        // Generic message
        Box::new(GenericMessage::new(self.payload, headers_to_use))
    }

    /// Create a MessageBuilder from an existing message.
    pub fn from_message(message: Box<dyn Message<T>>) -> Self {
        Self::from_provided_message(message)
    }

    /// Create a MessageBuilder with the given payload.
    pub fn with_payload(payload: T) -> Self {
        Self::with_payload_and_accessor(payload, MessageHeaderAccessor::new())
    }

    /// Create a message with the given payload and message headers.
    pub fn create_message(payload: T, message_headers: MessageHeaders) -> Box<dyn Message<T>>
    where
        T: Send + Sync,
        T: Clone,
        T: 'static,
    {
        // Generic message
        Box::new(GenericMessage::new(Some(payload), message_headers))
    }
}

impl<T> Debug for MessageBuilder<T>
where
    T: Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MessageBuilder")
            .field("provided_message", &self.provided_message.is_some())
            .field("header_accessor", &self.header_accessor)
            .finish_non_exhaustive()
    }
}
