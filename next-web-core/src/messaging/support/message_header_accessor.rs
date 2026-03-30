use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use uuid::Uuid;

use crate::anys::any_value::AnyValue;
use crate::messaging::message_channel::MessageChannel;
use crate::messaging::message_headers::MessageHeaders;
use crate::traits::any_clone::AnyClone;
use crate::traits::message::Message;
use crate::util::mime_type::MimeType;
use crate::util::pattern_match::PatternMatchUtils;

/// A class representing message header accessor.
pub struct MessageHeaderAccessor {
    headers: MutableMessageHeaders,
    leave_mutable: bool,
    modified: bool,
    enable_timestamp: bool,
    id_generator: Option<Uuid>,

    readable_mime_types: Vec<MimeType>,
}

impl MessageHeaderAccessor {
    pub fn new() -> Self {
        let headers = MutableMessageHeaders::default();
        Self {
            headers,
            leave_mutable: false,
            modified: false,
            enable_timestamp: false,
            id_generator: None,
            readable_mime_types: vec![
                MimeType::with_subtype("text", "*"),
                MimeType::with_subtype("application", "*+json"),
                MimeType::with_subtype("application", "*+xml"),
            ],
        }
    }
    /// Create a MessageHeaderAccessor from an existing message.
    pub fn from_message<T>(message: Box<dyn Message<T>>) -> Self {
        let headers = MutableMessageHeaders::new(message.get_headers().raw_headers());

        MessageHeaderAccessor {
            headers,
            leave_mutable: false,
            modified: false,
            enable_timestamp: false,
            id_generator: None,

            readable_mime_types: vec![
                MimeType::with_subtype("text", "*"),
                MimeType::with_subtype("application", "*+json"),
                MimeType::with_subtype("application", "*+xml"),
            ],
        }
    }

    /// Create a new accessor for the given message.
    pub fn create_accessor<T>(&self, message: Box<dyn Message<T>>) -> Self {
        Self::from_message(message)
    }

    /// Set whether to leave mutable.
    pub fn set_leave_mutable(&mut self, leave_mutable: bool) {
        assert!(self.headers.is_mutable(), "Already immutable");
        self.leave_mutable = leave_mutable;
    }

    /// Set to immutable.
    pub fn set_immutable(&mut self) {
        self.headers.set_immutable();
    }

    /// Check if mutable.
    pub fn is_mutable(&self) -> bool {
        self.headers.is_mutable()
    }

    /// Set modified flag.
    pub fn set_modified(&mut self, modified: bool) {
        self.modified = modified;
    }

    /// Check if modified.
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Enable timestamp.
    pub fn set_enable_timestamp(&mut self, enable_timestamp: bool) {
        self.enable_timestamp = enable_timestamp;
    }

    /// Set ID generator.
    pub fn set_id_generator(&mut self, id_generator: Uuid) {
        self.id_generator = Some(id_generator);
    }

    /// Get message headers.
    #[allow(unused)]
    pub(self) fn get_message_headers(&mut self) -> &mut MutableMessageHeaders {
        if !self.leave_mutable {
            self.set_immutable();
        }
        &mut self.headers
    }

    /// Convert to message headers.
    pub fn to_message_headers(&self) -> MessageHeaders {
        MessageHeaders::from(self.headers.get_own_raw_headers())
    }

    /// Convert to map.
    pub fn to_map(&self) -> HashMap<String, AnyValue> {
        self.headers.get_raw_headers().clone()
    }

    /// Get header value.
    pub fn get_header(&self, header_name: &str) -> Option<&AnyValue> {
        self.headers.get_raw_headers().get(header_name)
    }

    /// Set header value.
    pub fn set_header(&mut self, name: String, value: AnyValue) {
        if self.is_read_only(&name) {
            panic!("'{}' header is read-only", name);
        }

        self.verify_type(&name, &value);

        self.modified = true;
        self.headers.get_raw_headers_mut().insert(name, value);
    }

    /// Set header if absent.
    pub fn set_header_if_absent(&mut self, name: String, value: AnyValue) {
        if self.get_header(&name).is_none() {
            self.set_header(name, value);
        }
    }

    /// Verify header type.
    fn verify_type(&self, header_name: &str, header_value: &AnyValue) {
        if header_name == "errorChannel" || header_name == "replyChannel" {
            if !header_value.is_string() {
                panic!(
                    "'{}' header value must be a MessageChannel or String",
                    header_name
                );
            }
        }
    }

    /// Remove header.
    pub fn remove_header(&mut self, header_name: &str) {
        if !header_name.is_empty() && !self.is_read_only(header_name) {
            self.remove_header(header_name);
        }
    }

    /// Remove headers matching patterns.
    pub fn remove_headers(&mut self, header_patterns: Vec<String>) {
        let mut headers_to_remove = Vec::new();

        for pattern in header_patterns {
            if !pattern.is_empty() {
                if pattern.contains('*') {
                    headers_to_remove.extend(self.get_matching_header_names(&pattern));
                } else {
                    headers_to_remove.push(pattern);
                }
            }
        }

        for header in headers_to_remove {
            self.remove_header(&header);
        }
    }

    /// Get matching header names.
    fn get_matching_header_names(&self, pattern: &str) -> Vec<String> {
        let mut matching = Vec::new();
        for key in self.headers.get_raw_headers().keys() {
            if PatternMatchUtils::simple_match(pattern, key) {
                matching.push(key.clone());
            }
        }
        matching
    }

    /// Copy headers.
    pub fn copy_headers(&mut self, headers: HashMap<String, AnyValue>) {
        for (key, value) in headers {
            if !self.is_read_only(&key) {
                self.set_header(key, value);
            }
        }
    }

    /// Copy headers if absent.
    pub fn copy_headers_if_absent(&mut self, headers: HashMap<String, AnyValue>) {
        for (key, value) in headers {
            if !self.is_read_only(&key) {
                if self.get_header(&key).is_none() {
                    self.set_header(key, value);
                }
            }
        }
    }

    /// Check if header is read-only.
    fn is_read_only(&self, header_name: &str) -> bool {
        header_name == "id" || header_name == "timestamp"
    }

    /// Get ID.
    pub fn get_id(&self) -> Option<String> {
        self.get_header("id").and_then(|v| v.as_string())
    }

    /// Get timestamp.
    pub fn get_timestamp(&self) -> Option<u64> {
        self.get_header("timestamp").and_then(|value| {
            if let Some(ts) = value.as_string() {
                ts.parse().ok()
            } else if let Some(ts) = value.as_number() {
                ts.try_into().ok()
            } else {
                None
            }
        })
    }

    /// Set content type.
    pub fn set_content_type(&mut self, content_type: impl Into<String>) {
        self.set_header(
            "contentType".to_string(),
            AnyValue::String(content_type.into()),
        );
    }

    /// Get content type.
    pub fn get_content_type(&self) -> Option<&MimeType> {
        self.get_header("contentType")
            .and_then(|v| v.as_ref_object::<MimeType>())
    }

    /// Set reply channel name.
    pub fn set_reply_channel_name(&mut self, reply_channel_name: String) {
        self.set_header(
            "replyChannel".to_string(),
            AnyValue::String(reply_channel_name),
        );
    }

    /// Set reply channel.
    pub fn set_reply_channel<T1, T>(&mut self, reply_channel: T1)
    where
        T1: MessageChannel<T> + AnyClone,
        T: Send + Sync,
    {
        self.set_header(
            "replyChannel".to_string(),
            AnyValue::Object(Box::new(reply_channel)),
        );
    }

    /// Get reply channel.
    pub fn get_reply_channel(&self) -> Option<&AnyValue> {
        self.get_header("replyChannel")
    }

    /// Set error channel name.
    pub fn set_error_channel_name(&mut self, error_channel_name: String) {
        self.set_header(
            "errorChannel".to_string(),
            AnyValue::String(error_channel_name),
        );
    }

    /// Set error channel.
    pub fn set_error_channel<T1, T>(&mut self, error_channel: T1)
    where
        T1: MessageChannel<T> + AnyClone,
        T: Send + Sync,
    {
        self.set_header(
            "errorChannel".to_string(),
            AnyValue::Object(Box::new(error_channel)),
        );
    }

    /// Get error channel.
    pub fn get_error_channel(&self) -> Option<&AnyValue> {
        self.get_header("errorChannel")
    }

    /// Get short log message.
    pub fn get_short_log_message(&self, payload: &dyn Any) -> String {
        format!(
            "headers={} {}",
            self.headers,
            self.get_short_payload_log_message(payload)
        )
    }

    /// Get detailed log message.
    pub fn get_detailed_log_message(&self, payload: &dyn Any) -> String {
        format!(
            "headers={} {}",
            self.headers,
            self.get_detailed_payload_log_message(payload)
        )
    }

    /// Get short payload log message.
    fn get_short_payload_log_message(&self, payload: &dyn Any) -> String {
        if let Some(s) = payload.downcast_ref::<String>() {
            if s.len() < 80 {
                format!(" payload={}", s)
            } else {
                format!(" payload={}...(truncated)", &s[0..80])
            }
        } else if let Some(bytes) = payload.downcast_ref::<Vec<u8>>() {
            if self.is_readable_content_type() {
                let s = String::from_utf8_lossy(bytes);
                if bytes.len() < 80 {
                    format!(" payload={}", s)
                } else {
                    format!(" payload={}...(truncated)", &s[0..80])
                }
            } else {
                format!(" payload=byte[{}]", bytes.len())
            }
        } else {
            let s = format!("{:?}", payload);
            if s.len() < 80 {
                format!(" payload={}", s)
            } else {
                format!(" payload={:?}", payload)
            }
        }
    }

    /// Get detailed payload log message.
    fn get_detailed_payload_log_message(&self, payload: &dyn Any) -> String {
        if let Some(s) = payload.downcast_ref::<String>() {
            format!(" payload={}", s)
        } else if let Some(bytes) = payload.downcast_ref::<Vec<u8>>() {
            if self.is_readable_content_type() {
                format!(" payload={}", String::from_utf8_lossy(bytes))
            } else {
                format!(" payload=byte[{}]", bytes.len())
            }
        } else {
            format!(" payload={:?}", payload)
        }
    }

    /// Check if content type is readable.
    fn is_readable_content_type(&self) -> bool {
        if let Some(content_type) = self.get_content_type() {
            for mime in self.readable_mime_types.iter() {
                if mime.includes(content_type) {
                    return true;
                }
            }
        }
        false
    }

    /// Get mutable accessor.
    pub fn get_mutable_accessor<T>(message: Box<dyn Message<T>>) -> MessageHeaderAccessor {
        MessageHeaderAccessor::from_message(message)
    }
}

impl Clone for MessageHeaderAccessor {
    fn clone(&self) -> Self {
        MessageHeaderAccessor {
            headers: self.headers.clone(),
            leave_mutable: self.leave_mutable,
            modified: self.modified,
            enable_timestamp: self.enable_timestamp,
            id_generator: self.id_generator.clone(),

            readable_mime_types: self.readable_mime_types.clone(),
        }
    }
}

impl fmt::Display for MessageHeaderAccessor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MessageHeaderAccessor [headers={}]", self.headers)
    }
}

impl fmt::Debug for MessageHeaderAccessor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MessageHeaderAccessor")
            .field("headers", &self.headers)
            .field("leave_mutable", &self.leave_mutable)
            .field("modified", &self.modified)
            .field("enable_timestamp", &self.enable_timestamp)
            .finish()
    }
}

/// Mutable message headers implementation.
#[derive(Clone)]
struct MutableMessageHeaders {
    raw_headers: HashMap<String, AnyValue>,
    mutable: Arc<AtomicBool>,
    accessor: Option<*mut MessageHeaderAccessor>,
}

impl MutableMessageHeaders {
    fn new(headers: HashMap<String, AnyValue>) -> Self {
        Self {
            raw_headers: headers,
            mutable: Arc::new(AtomicBool::new(true)),
            accessor: None,
        }
    }

    fn get_raw_headers(&self) -> &HashMap<String, AnyValue> {
        &self.raw_headers
    }

    fn get_own_raw_headers(&self) -> HashMap<String, AnyValue> {
        self.raw_headers.clone()
    }

    fn get_raw_headers_mut(&mut self) -> &mut HashMap<String, AnyValue> {
        assert!(self.is_mutable(), "Already immutable");
        &mut self.raw_headers
    }

    fn set_immutable(&mut self) {
        if self.is_mutable() {
            if !self.raw_headers.contains_key("id") {
                // TODO: Generate ID
                // self.raw_headers.insert("id".to_string(), Box::new(Uuid::new_v4()));
            }

            if !self.raw_headers.contains_key("timestamp") && self.accessor.is_some() {
                // TODO: Check enable_timestamp flag
                // let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
                // self.raw_headers.insert("timestamp".to_string(), Box::new(now));
            }

            self.mutable.store(false, Ordering::Release);
        }
    }

    fn is_mutable(&self) -> bool {
        self.mutable.load(Ordering::Acquire)
    }

    #[allow(unused)]
    fn set_accessor(&mut self, accessor: *mut MessageHeaderAccessor) {
        self.accessor = Some(accessor);
    }
}

impl fmt::Display for MutableMessageHeaders {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.raw_headers)
    }
}

impl Default for MutableMessageHeaders {
    fn default() -> Self {
        Self {
            raw_headers: Default::default(),
            mutable: Arc::new(AtomicBool::new(true)),
            accessor: None,
        }
    }
}
impl fmt::Debug for MutableMessageHeaders {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MutableMessageHeaders")
            .field("raw_headers", &self.raw_headers)
            .field("mutable", &self.mutable.load(Ordering::Acquire))
            .finish()
    }
}
