use std::collections::HashMap;

use dyn_clone::clone_box;

use crate::anys::any_error::AnyError;
use crate::anys::any_value::AnyValue;
use crate::messaging::generic_message::GenericMessage;
use crate::messaging::message_headers::MessageHeaders;
use crate::traits::message::Message;

/// A message that carries an error payload.
///
/// `ErrorMessage` extends `GenericMessage` with a `Throwable` payload and optionally
/// holds a reference to the original message that caused the error.
pub struct ErrorMessage<T> {
    generic: GenericMessage<Box<dyn AnyError>>,
    original_message: Option<Box<dyn Message<T>>>,
}

impl<T> ErrorMessage<T> {
    /// Create a new ErrorMessage with the given payload.
    pub fn new(payload: Box<dyn AnyError>) -> Self {
        ErrorMessage {
            generic: GenericMessage::new(Some(payload), MessageHeaders::default()),
            original_message: None,
        }
    }

    /// Create a new ErrorMessage with the given payload and headers.
    pub fn with_headers(payload: Box<dyn AnyError>, headers: MessageHeaders) -> Self {
        ErrorMessage {
            generic: GenericMessage::new(Some(payload), headers),
            original_message: None,
        }
    }

    /// Create a new ErrorMessage with the given payload and header map.
    pub fn with_header_map(payload: Box<dyn AnyError>, headers: HashMap<String, AnyValue>) -> Self {
        ErrorMessage {
            generic: GenericMessage::new(Some(payload), MessageHeaders::from(headers)),
            original_message: None,
        }
    }

    /// Create a new ErrorMessage with the given payload and original message.
    pub fn with_original(
        payload: Box<dyn AnyError>,
        original_message: Box<dyn Message<T>>,
    ) -> Self {
        ErrorMessage {
            generic: GenericMessage::new(Some(payload), MessageHeaders::default()),
            original_message: Some(original_message),
        }
    }

    /// Create a new ErrorMessage with the given payload, headers, and original message.
    pub fn with_headers_and_original(
        payload: Box<dyn AnyError>,
        headers: MessageHeaders,
        original_message: Box<dyn Message<T>>,
    ) -> Self {
        ErrorMessage {
            generic: GenericMessage::new(Some(payload), headers),
            original_message: Some(original_message),
        }
    }

    /// Create a new ErrorMessage with the given payload, header map, and original message.
    pub fn with_header_map_and_original(
        payload: Box<dyn AnyError>,
        headers: HashMap<String, AnyValue>,
        original_message: Box<dyn Message<T>>,
    ) -> Self {
        ErrorMessage {
            generic: GenericMessage::new(Some(payload), MessageHeaders::from(headers)),
            original_message: Some(original_message),
        }
    }

    /// Get the original message that caused this error.
    pub fn get_original_message(&self) -> Option<&dyn Message<T>> {
        self.original_message.as_deref()
    }

    /// Get the original message as a boxed trait object.
    pub fn into_original_message(self) -> Option<Box<dyn Message<T>>> {
        self.original_message
    }
}

impl<T> Message<Box<dyn AnyError>> for ErrorMessage<T> {
    fn get_payload(&self) -> Option<&Box<dyn AnyError>> {
        self.generic.get_payload()
    }

    fn get_own_payload(&mut self) -> Option<Box<dyn AnyError>> {
        self.generic.get_own_payload()
    }

    fn get_headers(&self) -> &MessageHeaders {
        self.generic.get_headers()
    }
}

impl<T> Clone for ErrorMessage<T> {
    fn clone(&self) -> Self {
        ErrorMessage {
            generic: self.generic.clone(),
            original_message: self
                .original_message
                .as_ref()
                .map(|msg| clone_box(msg.as_ref())),
        }
    }
}
